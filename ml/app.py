import datetime
import numpy as np
import pandas as pd
import streamlit as st
import plotly.graph_objects as go
from plotly.subplots import make_subplots
from st_keyup import st_keyup

from pipeline import (
    DEFAULT_DATASET,
    FEATURE_COLS,
    SENSOR_MAC_1,
    SENSOR_MAC_2,
    create_windows,
    detect_hits,
    delete_last_samples,
    delete_samples_by_id,
    delete_samples_by_label,
    get_latest_sample_per_sensor,
    get_recent_samples,
    relabel_samples_by_label,
    load_data,
    load_dataset,
    merge_sensors,
    save_dataset,
)

# Tiempos para clasificar el estado del sensor (en segundos, sobre la edad del
# último sample). Verde si llega tráfico fresco, ámbar si va lento, rojo si
# nada en > 10s o no hay datos en absoluto.
SENSOR_OK_MAX_AGE_S = 2.0
SENSOR_WARN_MAX_AGE_S = 10.0

PUNCH_TYPES = ["jab", "cross", "hook", "uppercut"]
POSITIONS = [
    "izquierda_arriba",
    "izquierda_abajo",
    "frente_arriba",
    "frente_abajo",
    "derecha_arriba",
    "derecha_abajo",
]

# Atajos de teclado
PUNCH_SHORTCUTS = {
    "1": "jab",
    "2": "cross",
    "3": "hook",
    "4": "uppercut",
}
POSITION_SHORTCUTS = {
    "q": "izquierda_arriba",
    "w": "izquierda_abajo",
    "e": "frente_arriba",
    "r": "frente_abajo",
    "t": "derecha_arriba",
    "y": "derecha_abajo",
}


def init_session():
    # Umbral por defecto para el slider de detección. Distinto y más bajo que
    # HIT_THRESHOLD_G (el del backend de inferencia/entrenamiento) a propósito:
    # la herramienta de etiquetado quiere ver TODO impacto candidato para que
    # el humano decida, mientras que la inferencia online prefiere ser más
    # restrictiva. Mantenerlos desacoplados evita que tocar uno mueva el otro.
    DEFAULT_SLIDER_THRESHOLD = 1.2
    defaults = {
        "recording": False,
        "start_time": None,
        "end_time": None,
        "merged_df": None,
        "peaks": None,
        "windows": None,
        "threshold": DEFAULT_SLIDER_THRESHOLD,
        "last_threshold": DEFAULT_SLIDER_THRESHOLD,
        "default_punch": PUNCH_TYPES[0],
        "default_position": POSITIONS[0],
        "last_key": "",
    }
    for k, v in defaults.items():
        if k not in st.session_state:
            st.session_state[k] = v


st.set_page_config(page_title="KnockShadow Dataset Tool", layout="wide")
st.title("KnockShadow — Herramienta de Dataset")

init_session()


@st.fragment(run_every="2s")
def render_sensor_status() -> None:
    """Badge por sensor coloreado según la edad del último sample. Se redibuja
    cada 2 s sin re-ejecutar el script entero, así no interrumpe la grabación."""
    now = datetime.datetime.now(datetime.timezone.utc)
    try:
        latest = get_latest_sample_per_sensor()
    except Exception as exc:
        st.warning(f"No se pudo consultar el estado de los sensores: {exc}")
        return

    for mac, name in [(SENSOR_MAC_1, "Sensor 1"), (SENSOR_MAC_2, "Sensor 2")]:
        last = latest.get(mac)
        if last is None:
            color, status = "#d62728", "sin datos"
        else:
            age = (now - last.to_pydatetime()).total_seconds()
            if age < SENSOR_OK_MAX_AGE_S:
                color, status = "#2ca02c", f"OK · {age:.1f} s"
            elif age < SENSOR_WARN_MAX_AGE_S:
                color, status = "#ff8c00", f"lento · {age:.1f} s"
            else:
                color, status = "#d62728", f"inactivo · {age:.0f} s"
        st.markdown(
            (
                f'<div style="background:{color};color:#fff;padding:6px 10px;'
                'border-radius:6px;margin:4px 0;font-weight:600;font-size:0.9rem;">'
                f"● {name} — {status}<br>"
                f'<span style="font-weight:400;font-size:0.75rem;opacity:0.85;">'
                f"{mac}</span></div>"
            ),
            unsafe_allow_html=True,
        )


# ---- Sidebar ----
with st.sidebar:
    st.subheader("Sensores")
    render_sensor_status()
    st.divider()
    st.header("Dataset")
    X_ds, y_ds, ids_ds = load_dataset(DEFAULT_DATASET)
    if len(y_ds) > 0:
        st.metric("Total muestras", len(y_ds))
        counts = pd.Series(y_ds).value_counts().rename("muestras")
        st.dataframe(counts, use_container_width=True)
    else:
        st.info("Dataset vacío")

    st.divider()
    st.subheader("Últimas muestras guardadas")
    recent = get_recent_samples(n=10)
    if recent:
        for i, sample in enumerate(recent):
            cols = st.columns([3, 1])
            with cols[0]:
                st.caption(f"{sample['label']}")
                st.text(f"ID: {sample['id'][:8]}...")
            with cols[1]:
                if st.button(
                    "🗑️", key=f"del_{sample['id']}", help="Eliminar esta muestra"
                ):
                    delete_samples_by_id([sample["id"]])
                    st.success("Muestra eliminada")
                    st.rerun()
        st.divider()
        col_del1, col_del2 = st.columns(2)
        with col_del1:
            if st.button("Eliminar última", use_container_width=True):
                delete_last_samples(1)
                st.success("Última muestra eliminada")
                st.rerun()
        with col_del2:
            n_del = st.number_input(
                "N", min_value=1, max_value=50, value=1, label_visibility="collapsed"
            )
            if st.button(f"Eliminar últimas {n_del}", use_container_width=True):
                delete_last_samples(int(n_del))
                st.success(f"Eliminadas {n_del} muestras")
                st.rerun()
    else:
        st.info("No hay muestras guardadas")

    # ---- Gestionar por etiqueta ----
    st.divider()
    st.subheader("Gestionar por etiqueta")
    if len(y_ds) > 0:
        unique_labels = sorted(pd.Series(y_ds).unique().tolist())
        manage_label = st.selectbox(
            "Etiqueta",
            unique_labels,
            key="label_manage_select",
        )
        label_count = int((y_ds == manage_label).sum())
        st.caption(f"Hay **{label_count}** muestras con esta etiqueta")

        action = st.radio(
            "Acción",
            ["Eliminar", "Re-etiquetar"],
            key="label_action_radio",
            horizontal=True,
        )

        if action == "Eliminar":
            if st.button(
                f"Eliminar todas las '{manage_label}'",
                type="secondary",
                use_container_width=True,
            ):
                removed, remaining = delete_samples_by_label(manage_label)
                st.success(
                    f"Eliminadas {removed} muestras de '{manage_label}'. "
                    f"Quedan {remaining} en total."
                )
                st.rerun()
        else:
            all_possible_labels = sorted(
                {f"{t}_{p}" for t in PUNCH_TYPES for p in POSITIONS}
            )
            new_label = st.selectbox(
                "Nueva etiqueta",
                [l for l in all_possible_labels if l != manage_label],
                key="label_relabel_select",
            )
            if st.button(
                f"Cambiar '{manage_label}' → '{new_label}'",
                type="primary",
                use_container_width=True,
            ):
                changed, remaining = relabel_samples_by_label(manage_label, new_label)
                st.success(
                    f"Re-etiquetadas {changed} muestras de '{manage_label}' a "
                    f"'{new_label}'. Total en dataset: {remaining}."
                )
                st.rerun()
    else:
        st.info("Dataset vacío")

    st.divider()
    st.subheader("Configuración")
    # Rango y paso pensados para etiquetado de impactos suaves a fuertes:
    # 0.5–5.0 G cubre desde toques flojos hasta ganchos sólidos sin saturar
    # el slider con valores irrelevantes (>5 G casi nunca filtra nada útil
    # porque la mayoría de impactos legítimos caen por debajo). Paso de 0.1
    # para ajuste fino.
    new_threshold = st.slider(
        "Umbral de detección (G)",
        min_value=0.5,
        max_value=5.0,
        value=float(st.session_state.threshold),
        step=0.1,
        format="%.1f",
        help="Magnitud mínima en G para considerar un golpe",
    )
    if new_threshold != st.session_state.last_threshold:
        st.session_state.threshold = new_threshold
        st.session_state.last_threshold = new_threshold
        if (
            st.session_state.merged_df is not None
            and not st.session_state.merged_df.empty
        ):
            peaks = detect_hits(st.session_state.merged_df, threshold=new_threshold)
            windows, valid_peaks = create_windows(
                st.session_state.merged_df, peaks, return_valid_peaks=True
            )
            st.session_state.peaks = valid_peaks
            st.session_state.windows = windows
            # El número de golpes cambia → fuerza reconstrucción de hits_df y
            # limpia el estado del data_editor para que no reaplique ediciones
            # antiguas sobre filas que ya no existen.
            st.session_state.hits_dirty = True
            for _k in ("hits_df", "hits_editor"):
                st.session_state.pop(_k, None)

    st.caption(f"Sensor 1: `{SENSOR_MAC_1}`")
    st.caption(f"Sensor 2: `{SENSOR_MAC_2}`")

# ---- Main layout ----
col_control, col_viz = st.columns([1, 2])

with col_control:
    st.subheader("Control")

    if st.session_state.recording:
        st.error("GRABANDO — pega el golpe y pulsa Detener")
        if st.button("Detener grabación", type="secondary", use_container_width=True):
            st.session_state.recording = False
            st.session_state.end_time = datetime.datetime.now(datetime.timezone.utc)
            st.session_state.merged_df = None
            st.session_state.peaks = None
            st.session_state.windows = None
            # Invalida la tabla de golpes de la grabación anterior: si no, al
            # extraer las nuevas ventanas `hits_df` conservaría las filas viejas
            # (con `golpe` apuntando a índices que ya no existen en `windows`)
            # y al guardar petaría con IndexError.
            st.session_state.hits_dirty = True
            for _k in ("hits_df", "hits_editor"):
                st.session_state.pop(_k, None)
            st.rerun()
    else:
        if st.button("Iniciar grabación", type="primary", use_container_width=True):
            st.session_state.recording = True
            st.session_state.start_time = datetime.datetime.now(datetime.timezone.utc)
            st.session_state.end_time = None
            st.session_state.merged_df = None
            st.session_state.peaks = None
            st.session_state.windows = None
            # Misma invalidación que en "Detener" — empezar una nueva grabación
            # debe descartar los golpes etiquetados anteriormente.
            st.session_state.hits_dirty = True
            for _k in ("hits_df", "hits_editor"):
                st.session_state.pop(_k, None)
            st.rerun()

    if st.session_state.start_time and st.session_state.end_time:
        duration = (
            st.session_state.end_time - st.session_state.start_time
        ).total_seconds()
        st.metric("Duración grabada", f"{duration:.1f} s")

    # ---- Atajos de teclado ----
    if not st.session_state.recording and st.session_state.end_time:
        st.divider()
        st.subheader("Atajos de teclado")
        st.caption("Haz clic en el campo de abajo y presiona la tecla deseada")
        key_pressed = st_keyup(
            "Tecla presionada",
            key="keypress_input",
            label_visibility="collapsed",
            placeholder="Presiona 1-4 (tipo) o Q,Y (posición)...",
        )
        if key_pressed and key_pressed != st.session_state.last_key:
            k = key_pressed[-1].lower() if key_pressed else ""
            st.session_state.last_key = key_pressed
            if k in PUNCH_SHORTCUTS:
                st.session_state.default_punch = PUNCH_SHORTCUTS[k]
                st.toast(f"Tipo seleccionado: {PUNCH_SHORTCUTS[k]}")
                st.rerun()
            elif k in POSITION_SHORTCUTS:
                st.session_state.default_position = POSITION_SHORTCUTS[k]
                st.toast(f"Posición seleccionada: {POSITION_SHORTCUTS[k]}")
                st.rerun()

        cols_atajos = st.columns(2)
        with cols_atajos[0]:
            st.markdown(
                "**Tipos**<br>1: jab<br>2: cross<br>3: hook<br>4: uppercut",
                unsafe_allow_html=True,
            )
        with cols_atajos[1]:
            st.markdown(
                "**Posiciones**<br>Q: izq. arriba<br>W: izq. abajo<br>E: frente arriba<br>R: frente abajo<br>T: der. arriba<br>Y: der. abajo",
                unsafe_allow_html=True,
            )

        st.divider()
        st.subheader("Valores por defecto")
        default_punch = st.selectbox(
            "Tipo por defecto",
            PUNCH_TYPES,
            index=PUNCH_TYPES.index(st.session_state.default_punch),
            key="default_punch_select",
        )
        default_position = st.selectbox(
            "Posición por defecto",
            POSITIONS,
            index=POSITIONS.index(st.session_state.default_position),
            key="default_position_select",
        )
        st.session_state.default_punch = default_punch
        st.session_state.default_position = default_position

# ---- Visualization & per-hit labeling ----
with col_viz:
    if not st.session_state.recording and st.session_state.end_time:
        st.subheader("Señal")

        # Load and process data only once per recording
        if st.session_state.merged_df is None:
            with st.spinner("Cargando datos de la base de datos..."):
                t_start = st.session_state.start_time - datetime.timedelta(
                    milliseconds=300
                )
                t_end = st.session_state.end_time + datetime.timedelta(milliseconds=300)
                raw = load_data(t_start, t_end)

            if raw.empty:
                st.error("No hay datos en ese intervalo. ¿Están los sensores activos?")
            else:
                merged = merge_sensors(raw)
                if merged.empty:
                    st.warning(
                        "No se pudieron sincronizar los dos sensores. "
                        "Comprueba que ambos MACs están enviando datos."
                    )
                else:
                    peaks = detect_hits(merged, threshold=st.session_state.threshold)
                    windows, valid_peaks = create_windows(
                        merged, peaks, return_valid_peaks=True
                    )
                    st.session_state.merged_df = merged
                    st.session_state.peaks = valid_peaks
                    st.session_state.windows = windows

        merged = st.session_state.merged_df
        if merged is not None and not merged.empty:
            peaks = st.session_state.peaks
            windows = st.session_state.windows

            fig = make_subplots(
                rows=3,
                cols=1,
                shared_xaxes=True,
                subplot_titles=("Magnitud media", "Sensor 1", "Sensor 2"),
                vertical_spacing=0.08,
            )

            # Magnitude row
            fig.add_trace(
                go.Scatter(
                    y=merged["mag"].values,
                    name="magnitud",
                    line=dict(color="orange", width=2),
                ),
                row=1,
                col=1,
            )
            if len(peaks) > 0:
                fig.add_trace(
                    go.Scatter(
                        x=list(peaks),
                        y=merged["mag"].values[peaks],
                        mode="markers+text",
                        name="golpes",
                        marker=dict(color="red", size=12, symbol="x"),
                        text=[str(i + 1) for i in range(len(peaks))],
                        textposition="top center",
                        textfont=dict(color="red", size=12),
                    ),
                    row=1,
                    col=1,
                )
            fig.add_hline(
                y=st.session_state.threshold,
                line_dash="dash",
                line_color="red",
                opacity=0.5,
                row=1,
                col=1,
            )

            # Sensor 1 axes
            for col_name, color in [
                ("x1", "#1f77b4"),
                ("y1", "#2ca02c"),
                ("z1", "#d62728"),
            ]:
                fig.add_trace(
                    go.Scatter(
                        y=merged[col_name].values,
                        name=col_name,
                        line=dict(color=color, width=1),
                    ),
                    row=2,
                    col=1,
                )

            # Sensor 2 axes
            for col_name, color in [
                ("x2", "#9467bd"),
                ("y2", "#8c564b"),
                ("z2", "#e377c2"),
            ]:
                fig.add_trace(
                    go.Scatter(
                        y=merged[col_name].values,
                        name=col_name,
                        line=dict(color=color, width=1),
                    ),
                    row=3,
                    col=1,
                )

            fig.update_layout(height=520, margin=dict(t=40, b=10), showlegend=True)
            st.plotly_chart(fig, use_container_width=True)

            if len(windows) == 0:
                st.warning(
                    "No se extrajo ninguna ventana válida. "
                    "Prueba a bajar el umbral en el panel lateral o pega con más fuerza."
                )
            else:
                st.divider()
                st.subheader(f"Golpes detectados: {len(windows)}")

                # ---- Construye hits_df una sola vez por grabación ----
                # La columna `etiqueta` derivada se calcula bajo demanda fuera
                # del data_editor: meterla dentro creaba un lag de un rerun
                # (cambiabas "tipo" y la etiqueta seguía mostrando el valor
                # anterior hasta que volvías a interactuar), que daba la falsa
                # sensación de que la selección no se aplicaba.
                # Defensa en profundidad: si por cualquier motivo `hits_df`
                # quedó desincronizado con `windows` (p. ej. un flujo nuevo que
                # olvide poner hits_dirty=True), reconstruye en vez de dejar
                # que `windows[indices]` reviente al guardar.
                _existing_hits = st.session_state.get("hits_df")
                _hits_len_mismatch = (
                    _existing_hits is not None and len(_existing_hits) != len(windows)
                )
                if (
                    "hits_df" not in st.session_state
                    or st.session_state.get("hits_dirty", True)
                    or _hits_len_mismatch
                ):
                    hits_data = []
                    for i in range(len(windows)):
                        peak_idx = int(peaks[i]) if i < len(peaks) else 0
                        mag_peak = (
                            float(merged["mag"].values[peak_idx])
                            if peak_idx < len(merged)
                            else 0.0
                        )
                        time_str = ""
                        if "received_at" in merged.columns and peak_idx < len(merged):
                            ts = merged["received_at"].iloc[peak_idx]
                            if pd.notna(ts):
                                time_str = ts.strftime("%H:%M:%S.%f")[:-3]
                        hits_data.append(
                            {
                                "guardar": True,
                                "golpe": i + 1,
                                "tiempo": time_str,
                                "pico_mag": round(mag_peak, 2),
                                "tipo": st.session_state.default_punch,
                                "posicion": st.session_state.default_position,
                            }
                        )
                    st.session_state.hits_df = pd.DataFrame(hits_data)
                    st.session_state.hits_dirty = False
                    # Imprescindible: si quedaba widget-state de una grabación
                    # anterior, Streamlit lo aplicaría sobre los nuevos golpes
                    # y reescribiría filas que el usuario no había tocado.
                    st.session_state.pop("hits_editor", None)

                def _sync_hits_editor() -> None:
                    """Aplica las ediciones del widget a hits_df.

                    Se llama tanto como `on_change` (antes del rerun, para que
                    el resto del script vea los cambios inmediatamente) como
                    de forma defensiva justo después de `st.data_editor` por
                    si on_change no se disparase en algún caso límite.
                    """
                    state = st.session_state.get("hits_editor")
                    if not state:
                        return
                    df = st.session_state.hits_df
                    for row_idx, changes in state.get("edited_rows", {}).items():
                        try:
                            ri = int(row_idx)
                        except (TypeError, ValueError):
                            continue
                        if 0 <= ri < len(df):
                            for col, val in changes.items():
                                if col in df.columns:
                                    df.at[ri, col] = val
                    st.session_state.hits_df = df

                # ---- Barra de acciones masivas ----
                st.markdown(
                    "**Acciones rápidas** — aplica a TODOS los golpes a la vez:"
                )
                bcols = st.columns([1.4, 1.4, 1, 1, 1])
                with bcols[0]:
                    bulk_tipo = st.selectbox(
                        "Tipo masivo",
                        PUNCH_TYPES,
                        index=PUNCH_TYPES.index(st.session_state.default_punch),
                        key="bulk_tipo_select",
                        label_visibility="collapsed",
                    )
                with bcols[1]:
                    bulk_pos = st.selectbox(
                        "Posición masiva",
                        POSITIONS,
                        index=POSITIONS.index(st.session_state.default_position),
                        key="bulk_pos_select",
                        label_visibility="collapsed",
                    )
                with bcols[2]:
                    if st.button(
                        "Aplicar tipo",
                        use_container_width=True,
                        help="Asigna ese tipo a TODOS los golpes",
                    ):
                        st.session_state.hits_df["tipo"] = bulk_tipo
                        st.session_state.pop("hits_editor", None)
                        st.rerun()
                with bcols[3]:
                    if st.button(
                        "Aplicar pos.",
                        use_container_width=True,
                        help="Asigna esa posición a TODOS los golpes",
                    ):
                        st.session_state.hits_df["posicion"] = bulk_pos
                        st.session_state.pop("hits_editor", None)
                        st.rerun()
                with bcols[4]:
                    if st.button(
                        "Invertir sel.",
                        use_container_width=True,
                        help="Invierte qué golpes están marcados para guardar",
                    ):
                        st.session_state.hits_df["guardar"] = ~st.session_state.hits_df[
                            "guardar"
                        ].astype(bool)
                        st.session_state.pop("hits_editor", None)
                        st.rerun()

                # ---- Editor por fila ----
                # OJO: pasamos una copia para que el widget no muta hits_df
                # directamente; las ediciones se aplican vía _sync_hits_editor.
                st.data_editor(
                    st.session_state.hits_df.copy(),
                    column_config={
                        "guardar": st.column_config.CheckboxColumn(
                            "✓",
                            help="Marca para guardar este golpe",
                            width="small",
                        ),
                        "golpe": st.column_config.NumberColumn(
                            "#", disabled=True, width="small"
                        ),
                        "tiempo": st.column_config.TextColumn(
                            "Tiempo",
                            disabled=True,
                            help="Momento aproximado del pico",
                        ),
                        "pico_mag": st.column_config.NumberColumn(
                            "Pico (G)", disabled=True, format="%.2f"
                        ),
                        "tipo": st.column_config.SelectboxColumn(
                            "Tipo", options=PUNCH_TYPES, required=True
                        ),
                        "posicion": st.column_config.SelectboxColumn(
                            "Posición", options=POSITIONS, required=True
                        ),
                    },
                    hide_index=True,
                    use_container_width=True,
                    key="hits_editor",
                    on_change=_sync_hits_editor,
                )
                # Defensa por si on_change no se disparase (cambios programáticos).
                _sync_hits_editor()

                current = st.session_state.hits_df
                current_labels = (
                    current["tipo"].astype(str) + "_" + current["posicion"].astype(str)
                )
                to_save_mask = current["guardar"].astype(bool)
                n_selected = int(to_save_mask.sum())

                # ---- Resumen vivo por etiqueta (chips de colores) ----
                counts = current_labels[to_save_mask].value_counts()
                if len(counts) > 0:
                    palette = [
                        "#1f77b4",
                        "#ff7f0e",
                        "#2ca02c",
                        "#d62728",
                        "#9467bd",
                        "#8c564b",
                        "#e377c2",
                        "#7f7f7f",
                        "#bcbd22",
                        "#17becf",
                    ]
                    chips_html = []
                    for i, label in enumerate(counts.index):
                        color = palette[i % len(palette)]
                        chips_html.append(
                            f'<span style="background:{color};color:#fff;'
                            "padding:3px 10px;border-radius:12px;margin:2px;"
                            'display:inline-block;font-size:0.85rem;'
                            f'font-weight:600;">{label} · {int(counts[label])}</span>'
                        )
                    st.markdown(
                        '<div style="margin:6px 0 12px 0;">'
                        + "".join(chips_html)
                        + "</div>",
                        unsafe_allow_html=True,
                    )

                # ---- Inspector de golpe individual ----
                with st.expander(
                    f"🔍 Inspeccionar un golpe ({len(windows)} disponibles)",
                    expanded=False,
                ):
                    inspect_n = st.number_input(
                        "Número de golpe",
                        min_value=1,
                        max_value=int(len(windows)),
                        value=1,
                        step=1,
                        key="hit_inspector_idx",
                    )
                    idx_i = int(inspect_n) - 1
                    if 0 <= idx_i < len(windows):
                        w = windows[idx_i]
                        fig_i = make_subplots(
                            rows=2,
                            cols=1,
                            shared_xaxes=True,
                            subplot_titles=("Sensor 1", "Sensor 2"),
                            vertical_spacing=0.12,
                        )
                        for ax_i, color, name in [
                            (0, "#1f77b4", "x1"),
                            (1, "#2ca02c", "y1"),
                            (2, "#d62728", "z1"),
                        ]:
                            fig_i.add_trace(
                                go.Scatter(
                                    y=w[:, ax_i],
                                    name=name,
                                    line=dict(color=color, width=1.5),
                                ),
                                row=1,
                                col=1,
                            )
                        for ax_i, color, name in [
                            (3, "#9467bd", "x2"),
                            (4, "#8c564b", "y2"),
                            (5, "#e377c2", "z2"),
                        ]:
                            fig_i.add_trace(
                                go.Scatter(
                                    y=w[:, ax_i],
                                    name=name,
                                    line=dict(color=color, width=1.5),
                                ),
                                row=2,
                                col=1,
                            )
                        fig_i.update_layout(
                            height=320, margin=dict(t=40, b=10), showlegend=True
                        )
                        st.plotly_chart(fig_i, use_container_width=True)
                        row_i = current.iloc[idx_i]
                        flag = "✓ se guardará" if bool(row_i["guardar"]) else "✗ NO se guardará"
                        st.caption(
                            f"Etiqueta: **{row_i['tipo']}_{row_i['posicion']}** · "
                            f"Pico: **{row_i['pico_mag']} G** · {flag}"
                        )

                st.metric("Golpes seleccionados para guardar", n_selected)

                if st.button(
                    f"Guardar {n_selected} golpe(s) seleccionado(s)",
                    type="primary",
                    use_container_width=True,
                    disabled=(n_selected == 0),
                ):
                    selected = current[to_save_mask]
                    indices = selected["golpe"].values.astype(int) - 1
                    X_to_save = windows[indices]
                    y_to_save = (
                        selected["tipo"].astype(str)
                        + "_"
                        + selected["posicion"].astype(str)
                    ).values.astype(str)
                    total, _ = save_dataset(X_to_save, y_to_save)
                    st.success(
                        f"Guardadas {n_selected} muestra(s) — total en dataset: {total}"
                    )
                    # Reset for next recording
                    st.session_state.end_time = None
                    st.session_state.merged_df = None
                    st.session_state.peaks = None
                    st.session_state.windows = None
                    st.session_state.hits_dirty = True
                    for _k in ("hits_df", "hits_editor"):
                        st.session_state.pop(_k, None)
                    st.rerun()

                if st.button(
                    "Descartar y nueva grabación",
                    type="secondary",
                    use_container_width=True,
                ):
                    st.session_state.end_time = None
                    st.session_state.merged_df = None
                    st.session_state.peaks = None
                    st.session_state.windows = None
                    st.session_state.hits_dirty = True
                    for _k in ("hits_df", "hits_editor"):
                        st.session_state.pop(_k, None)
                    st.rerun()
