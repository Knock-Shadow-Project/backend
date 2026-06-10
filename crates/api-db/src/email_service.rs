//! Servicio de envío de emails transaccionales vía Resend.
//!
//! Aísla al resto del crate del SDK específico (`resend-rs`) para que un
//! futuro cambio de proveedor (Mailgun, Postmark, AWS SES, etc.) sea una
//! sustitución de implementación sin tocar los handlers HTTP.
//!
//! Diseño deliberado:
//!  - El cliente de Resend es **clonable y seguro en async**; lo metemos
//!    detrás de un `Arc` dentro de `AppState` y lo clonamos por handler.
//!  - El envío se hace **fire-and-forget** desde los handlers (`tokio::spawn`)
//!    para no añadir 200-800ms de latencia al endpoint público. Si Resend
//!    falla lo veremos en `tracing::error!` pero la respuesta HTTP ya
//!    habrá vuelto al cliente.
//!  - El servicio sólo se construye si hay `RESEND_API_KEY`; sin clave,
//!    `EmailService::from_config` devuelve `None` y los handlers caen al
//!    modo stub (loggean el enlace en vez de mandarlo).

use resend_rs::types::CreateEmailBaseOptions;
use resend_rs::{Resend, Result as ResendResult};

use crate::config::Config;

/// Wrapper del cliente Resend con la configuración del producto inyectada.
#[derive(Clone)]
pub struct EmailService {
    client: Resend,
    from: String,
    app_base_url: String,
}

impl std::fmt::Debug for EmailService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Nunca volcamos `client` (puede llevar la API key en su Debug
        // según la versión del SDK). Sólo metadatos del producto.
        f.debug_struct("EmailService")
            .field("from", &self.from)
            .field("app_base_url", &self.app_base_url)
            .finish()
    }
}

impl EmailService {
    /// Construye el servicio si el `Config` trae `resend_api_key`. Devuelve
    /// `None` cuando no hay clave: el caller debe interpretar eso como
    /// "modo stub activo" y loggear los enlaces en vez de enviarlos.
    pub fn from_config(cfg: &Config) -> Option<Self> {
        let key = cfg.resend_api_key.as_deref()?;
        Some(Self {
            client: Resend::new(key),
            from: cfg.resend_from.clone(),
            app_base_url: cfg.app_base_url.clone(),
        })
    }

    /// Envía el email de confirmación de cuenta con el token JWT incrustado.
    ///
    /// Devuelve el `email_id` que Resend asigna al mensaje (útil para
    /// trazar entregas en su dashboard).
    pub async fn send_confirmation(&self, destinatario: &str, token: &str) -> ResendResult<String> {
        let url = format!(
            "{}/confirm-email?token={}",
            self.app_base_url.trim_end_matches('/'),
            token
        );
        let subject = "Confirma tu correo en KnockShadow";
        let html = construir_html_confirmacion(destinatario, &url);
        let text = construir_texto_confirmacion(&url);

        let to = [destinatario];
        let options = CreateEmailBaseOptions::new(self.from.clone(), to, subject)
            .with_html(&html)
            .with_text(&text);

        let resp = self.client.emails.send(options).await?;
        Ok(resp.id.to_string())
    }

    pub async fn send_password_reset(
        &self,
        destinatario: &str,
        token: &str,
    ) -> ResendResult<String> {
        let url = format!(
            "{}/reset-password?token={}",
            self.app_base_url.trim_end_matches('/'),
            token
        );
        let subject = "Restablece tu contraseña en KnockShadow";
        let html = construir_html_reset_password(destinatario, &url);
        let text = construir_texto_reset_password(&url);

        let to = [destinatario];
        let options = CreateEmailBaseOptions::new(self.from.clone(), to, subject)
            .with_html(&html)
            .with_text(&text);

        let resp = self.client.emails.send(options).await?;
        Ok(resp.id.to_string())
    }
}

/// HTML del email de confirmación. Estilos inline para máxima compatibilidad
/// con clientes de correo (Gmail/Outlook/Apple Mail) que ignoran `<style>`.
fn construir_html_confirmacion(destinatario: &str, url: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="es">
  <head>
    <meta charset="utf-8" />
    <title>Confirma tu correo</title>
  </head>
  <body style="margin:0;padding:0;background-color:#131315;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Helvetica,Arial,sans-serif;color:#e5e1e4;">
    <table role="presentation" width="100%" cellpadding="0" cellspacing="0" style="background-color:#131315;padding:32px 16px;">
      <tr>
        <td align="center">
          <table role="presentation" width="100%" cellpadding="0" cellspacing="0" style="max-width:520px;background-color:#1b1b1d;border:1px solid #27272a;border-radius:12px;padding:32px;">
            <tr>
              <td style="padding-bottom:24px;border-bottom:1px solid #27272a;">
                <h1 style="margin:0;color:#ff525c;font-size:22px;font-weight:900;letter-spacing:3px;font-style:italic;">KNOCKSHADOW</h1>
                <p style="margin:6px 0 0;color:#71717a;font-size:11px;letter-spacing:2px;text-transform:uppercase;">Sistema de telemetría biométrica</p>
              </td>
            </tr>
            <tr>
              <td style="padding:24px 0 8px;">
                <h2 style="margin:0;color:#ffffff;font-size:24px;font-weight:900;">Confirma tu correo, atleta.</h2>
                <p style="margin:12px 0 0;color:#a1a1aa;font-size:14px;line-height:22px;">
                  Casi listo, <strong style="color:#ffffff;">{destinatario}</strong>. Pulsa el botón para activar tu cuenta y empezar a registrar telemetría del saco.
                </p>
              </td>
            </tr>
            <tr>
              <td align="center" style="padding:24px 0;">
                <a href="{url}" style="display:inline-block;background-color:#ff525c;color:#410008;font-weight:900;letter-spacing:2px;text-decoration:none;padding:14px 28px;border-radius:8px;font-size:14px;">CONFIRMAR CORREO</a>
              </td>
            </tr>
            <tr>
              <td style="padding-top:8px;">
                <p style="margin:0;color:#71717a;font-size:11px;line-height:18px;">
                  Si el botón no funciona, copia este enlace en tu navegador:<br />
                  <a href="{url}" style="color:#6cd7d8;word-break:break-all;">{url}</a>
                </p>
              </td>
            </tr>
            <tr>
              <td style="padding-top:24px;border-top:1px solid #27272a;">
                <p style="margin:0;color:#52525b;font-size:10px;letter-spacing:1px;text-align:center;text-transform:uppercase;">
                  Si no te registraste, ignora este correo. El enlace caduca en 24h.
                </p>
              </td>
            </tr>
          </table>
        </td>
      </tr>
    </table>
  </body>
</html>"#
    )
}

/// Texto plano del email — fallback para clientes sin HTML y para el
/// scoring de spam (los proveedores penalizan emails sin parte text).
fn construir_texto_confirmacion(url: &str) -> String {
    format!(
        "KNOCKSHADOW — Confirma tu correo\n\
         \n\
         Pulsa el siguiente enlace para activar tu cuenta:\n\
         {url}\n\
         \n\
         El enlace caduca en 24 horas. Si no te registraste, ignora este mensaje.\n"
    )
}

fn construir_html_reset_password(destinatario: &str, url: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="es">
  <head>
    <meta charset="utf-8" />
    <title>Restablece tu contraseña</title>
  </head>
  <body style="margin:0;padding:0;background-color:#131315;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Helvetica,Arial,sans-serif;color:#e5e1e4;">
    <table role="presentation" width="100%" cellpadding="0" cellspacing="0" style="background-color:#131315;padding:32px 16px;">
      <tr>
        <td align="center">
          <table role="presentation" width="100%" cellpadding="0" cellspacing="0" style="max-width:520px;background-color:#1b1b1d;border:1px solid #27272a;border-radius:12px;padding:32px;">
            <tr>
              <td style="padding-bottom:24px;border-bottom:1px solid #27272a;">
                <h1 style="margin:0;color:#ff525c;font-size:22px;font-weight:900;letter-spacing:3px;font-style:italic;">KNOCKSHADOW</h1>
                <p style="margin:6px 0 0;color:#71717a;font-size:11px;letter-spacing:2px;text-transform:uppercase;">Sistema de telemetría biométrica</p>
              </td>
            </tr>
            <tr>
              <td style="padding:24px 0 8px;">
                <h2 style="margin:0;color:#ffffff;font-size:24px;font-weight:900;">Restablece tu contraseña, atleta.</h2>
                <p style="margin:12px 0 0;color:#a1a1aa;font-size:14px;line-height:22px;">
                  Recibimos una solicitud para restablecer la contraseña de <strong style="color:#ffffff;">{destinatario}</strong>. Pulsa el botón para crear una nueva.
                </p>
              </td>
            </tr>
            <tr>
              <td align="center" style="padding:24px 0;">
                <a href="{url}" style="display:inline-block;background-color:#ff525c;color:#410008;font-weight:900;letter-spacing:2px;text-decoration:none;padding:14px 28px;border-radius:8px;font-size:14px;">RESTABLECER CONTRASEÑA</a>
              </td>
            </tr>
            <tr>
              <td style="padding-top:8px;">
                <p style="margin:0;color:#71717a;font-size:11px;line-height:18px;">
                  Si el botón no funciona, copia este enlace en tu navegador:<br />
                  <a href="{url}" style="color:#6cd7d8;word-break:break-all;">{url}</a>
                </p>
              </td>
            </tr>
            <tr>
              <td style="padding-top:24px;border-top:1px solid #27272a;">
                <p style="margin:0;color:#52525b;font-size:10px;letter-spacing:1px;text-align:center;text-transform:uppercase;">
                  Si no solicitaste este cambio, ignora este correo. El enlace caduca en 1h.
                </p>
              </td>
            </tr>
          </table>
        </td>
      </tr>
    </table>
  </body>
</html>"#
    )
}

fn construir_texto_reset_password(url: &str) -> String {
    format!(
        "KNOCKSHADOW — Restablece tu contraseña\n\
         \n\
         Pulsa el siguiente enlace para crear una nueva contraseña:\n\
         {url}\n\
         \n\
         El enlace caduca en 1 hora. Si no solicitaste este cambio, ignora este mensaje.\n"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_config_returns_none_without_key() {
        let cfg = Config {
            database_url: "postgres://x".into(),
            port: 3000,
            resend_api_key: None,
            resend_from: "x@y.com".into(),
            app_base_url: "https://example.com".into(),
            redis_url: None,
        };
        assert!(EmailService::from_config(&cfg).is_none());
    }

    #[test]
    fn from_config_returns_some_with_key() {
        let cfg = Config {
            database_url: "postgres://x".into(),
            port: 3000,
            resend_api_key: Some("re_test".into()),
            resend_from: "x@y.com".into(),
            app_base_url: "https://example.com".into(),
            redis_url: None,
        };
        let svc = EmailService::from_config(&cfg).unwrap();
        assert_eq!(svc.from, "x@y.com");
        assert_eq!(svc.app_base_url, "https://example.com");
    }

    #[test]
    fn html_contains_destinatario_and_url() {
        let html =
            construir_html_confirmacion("vic@example.com", "https://app/confirm-email?token=abc");
        assert!(html.contains("vic@example.com"));
        assert!(html.contains("https://app/confirm-email?token=abc"));
        // El botón usa los colores HUD del producto.
        assert!(html.contains("#ff525c"));
    }

    #[test]
    fn text_contains_url() {
        let text = construir_texto_confirmacion("https://app/confirm-email?token=abc");
        assert!(text.contains("https://app/confirm-email?token=abc"));
    }

    /// E2E real contra el sandbox de Resend.
    ///
    /// `#[ignore]` por defecto porque (a) consume cupo de la cuenta Resend
    /// y (b) requiere `RESEND_API_KEY` exportada al entorno del test, que
    /// CI no tiene de serie.
    ///
    /// Para correrlo a mano:
    /// ```bash
    /// export RESEND_API_KEY="re_..."   # idealmente una key con scope sandbox
    /// cargo test -p api-db --bin api-db -- --ignored email_service::tests::e2e
    /// ```
    ///
    /// Usa el destinatario mágico `delivered@resend.dev`: Resend acepta el
    /// mensaje, devuelve `email_id` real, pero NO entrega a nadie. Es la
    /// dirección recomendada por Resend para CI/E2E sin spamear inboxes.
    #[tokio::test]
    #[ignore]
    async fn e2e_send_confirmation_via_sandbox() {
        let Ok(api_key) = std::env::var("RESEND_API_KEY") else {
            println!("RESEND_API_KEY no definida — saltando E2E");
            return;
        };

        let cfg = Config {
            database_url: "postgres://x".into(),
            port: 3000,
            resend_api_key: Some(api_key),
            // Importante: usa un dominio verificado en la cuenta de Resend
            // que pertenezca a la API key. Si no, Resend devuelve 403.
            resend_from: std::env::var("RESEND_FROM")
                .unwrap_or_else(|_| "KnockShadow <no-reply@knockshadow.site>".into()),
            app_base_url: "https://api.knockshadow.site".into(),
            redis_url: None,
        };

        let svc = EmailService::from_config(&cfg).expect("EmailService debe construirse");
        let token = "test.jwt.token.fake.but.formatted.like.real";
        let result = svc.send_confirmation("delivered@resend.dev", token).await;

        match result {
            Ok(id) => {
                assert!(!id.is_empty(), "el id de Resend debe ser no vacío");
                println!("E2E OK — Resend devolvió email_id={id}");
            }
            Err(e) => panic!("E2E fallo: {e}"),
        }
    }
}
