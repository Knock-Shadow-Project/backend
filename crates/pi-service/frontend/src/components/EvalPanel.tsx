import { useEffect, useState } from "react";
import type { PunchEvent, ActiveTraining } from "../types";
import { describe, fmtAge } from "../utils";
import { IDLE_MS } from "../constants";

interface Props {
  recent: PunchEvent[];
  activeTraining: ActiveTraining | null;
  flashKey: number;
}

export function EvalPanel({ recent, activeTraining, flashKey }: Props) {
  const [, tick] = useState(0);

  useEffect(() => {
    const id = setInterval(() => tick((n) => n + 1), 1000);
    return () => clearInterval(id);
  }, []);

  const latest = recent[0] ?? null;
  const isIdle =
    activeTraining != null &&
    latest != null &&
    Date.now() - latest._ts > IDLE_MS;

  return (
    <section id="panel-eval" className="panel" role="tabpanel">
      <div className="eval-grid">
        <LastPunch punch={latest} isIdle={isIdle} flashKey={flashKey} />
        <Timeline recent={recent} />
      </div>
    </section>
  );
}

function LastPunch({
  punch,
  isIdle,
  flashKey,
}: {
  punch: PunchEvent | null;
  isIdle: boolean;
  flashKey: number;
}) {
  const [flashing, setFlashing] = useState(false);

  useEffect(() => {
    if (flashKey === 0) return;
    setFlashing(true);
    const id = setTimeout(() => setFlashing(false), 600);
    return () => clearTimeout(id);
  }, [flashKey]);

  if (!punch) {
    return (
      <article id="last-punch" className="empty">
        <div className="empty-state">Esperando el primer golpe...</div>
      </article>
    );
  }

  const d = describe(punch);
  const armClass =
    d.armKey === "izquierda"
      ? "chip arm-left"
      : d.armKey === "derecha"
        ? "chip arm-right"
        : "chip";

  return (
    <article id="last-punch" className={flashing ? "flash" : ""}>
      <div className="punch-detail">
        <div className="lp-label">Ultimo golpe</div>
        <h2 id="lp-type">{d.type}</h2>
        <div className="chips">
          <span className={armClass}>
            <span className="ico">{d.armIcon}</span> {d.arm}
          </span>
          <span className="chip">
            <span className="ico">{d.heightIcon}</span> {d.height}
          </span>
        </div>
        <dl className="meta">
          <div>
            <dt>Seguridad</dt>
            <dd>
              {Number.isFinite(punch.prob)
                ? `${(punch.prob * 100).toFixed(1)}%`
                : "—"}
            </dd>
          </div>
          <div>
            <dt>Potencia</dt>
            <dd>
              {punch.power != null && Number.isFinite(punch.power)
                ? `${(punch.power * 9.8).toFixed(2)} m/s²`
                : "—"}
            </dd>
          </div>
          <div>
            <dt>Tiempo</dt>
            <dd>{fmtAge(punch._ts)}</dd>
          </div>
        </dl>
        {isIdle && <span id="classifier-idle">Clasificador en espera</span>}
      </div>
    </article>
  );
}

function Timeline({ recent }: { recent: PunchEvent[] }) {
  return (
    <aside id="timeline">
      <h3>Historial</h3>
      <ul id="punch-list">
        {recent.length === 0 ? (
          <li className="muted">No hay golpes</li>
        ) : (
          recent.slice(0, 15).map((ev) => {
            const d = describe(ev);
            const probTxt = Number.isFinite(ev.prob)
              ? `${(ev.prob * 100).toFixed(0)}%`
              : "—";
            const powerTxt =
              ev.power != null && Number.isFinite(ev.power)
                ? `${ev.power.toFixed(2)} G`
                : "—";
            return (
              <li key={ev._ts}>
                <div className="tl-type">{d.type}</div>
                <div className="tl-meta">
                  {d.arm} · {d.height} · {probTxt} · {powerTxt}
                </div>
                <div className="tl-age">{fmtAge(ev._ts)}</div>
              </li>
            );
          })
        )}
      </ul>
    </aside>
  );
}
