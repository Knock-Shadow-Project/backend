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
    HIT_THRESHOLD_G,
    SENSOR_MAC_1,
    SENSOR_MAC_2,
    create_windows,
    detect_hits,
    delete_last_samples,
    delete_samples_by_id,
    delete_samples_by_label,
    get_recent_samples,
    relabel_samples_by_label,
    load_data,
    load_dataset,
    merge_sensors,
    save_dataset,
)

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
    defaults = {
        "recording": False,
        "start_time": None,
        "end_time": None,
        "merged_df": None,
        "peaks": None,
        "windows": None,
        "threshold": HIT_THRESHOLD_G,
        "last_threshold": HIT_THRESHOLD_G,
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

# ---- Sidebar ----
with st.sidebar:
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
    new_threshold = st.slider(
        "Umbral de detección (G)",
        min_value=1.0,
        max_value=20.0,
        value=float(st.session_state.threshold),
        step=0.5,
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
            st.rerun()
    else:
        if st.button("Iniciar grabación", type="primary", use_container_width=True):
            st.session_state.recording = True
            st.session_state.start_time = datetime.datetime.now(datetime.timezone.utc)
            st.session_state.end_time = None
            st.session_state.merged_df = None
            st.session_state.peaks = None
            st.session_state.windows = None
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

                # Build editable dataframe for each hit
                if "hits_df" not in st.session_state or st.session_state.get(
                    "hits_dirty", True
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

                df_editable = st.session_state.hits_df.copy()
                df_editable["etiqueta"] = (
                    df_editable["tipo"] + "_" + df_editable["posicion"]
                )

                edited_df = st.data_editor(
                    df_editable,
                    column_config={
                        "guardar": st.column_config.CheckboxColumn(
                            "Guardar", help="Marca para guardar este golpe"
                        ),
                        "golpe": st.column_config.NumberColumn("Golpe #", disabled=True),
                        "tiempo": st.column_config.TextColumn(
                            "Tiempo", disabled=True, help="Momento aproximado del pico"
                        ),
                        "pico_mag": st.column_config.NumberColumn(
                            "Pico mag (G)", disabled=True
                        ),
                        "tipo": st.column_config.SelectboxColumn(
                            "Tipo", options=PUNCH_TYPES, required=True
                        ),
                        "posicion": st.column_config.SelectboxColumn(
                            "Posición", options=POSITIONS, required=True
                        ),
                        "etiqueta": st.column_config.TextColumn(
                            "Etiqueta final", disabled=True
                        ),
                    },
                    hide_index=True,
                    use_container_width=True,
                    key="hits_editor",
                )

                # Persist user edits (recalculate label) so changes survive reruns
                if edited_df is not None and not edited_df.empty:
                    edited_df["etiqueta"] = (
                        edited_df["tipo"] + "_" + edited_df["posicion"]
                    )
                    st.session_state.hits_df = edited_df.copy()

                n_selected = (
                    int(edited_df["guardar"].sum())
                    if "guardar" in edited_df.columns
                    else 0
                )
                st.metric("Golpes seleccionados para guardar", n_selected)

                if st.button(
                    f"Guardar {n_selected} golpe(s) seleccionado(s)",
                    type="primary",
                    use_container_width=True,
                    disabled=(n_selected == 0),
                ):
                    selected = edited_df[edited_df["guardar"] == True]
                    indices = selected["golpe"].values.astype(int) - 1
                    X_to_save = windows[indices]
                    y_to_save = selected["etiqueta"].values.astype(str)
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
                    if "hits_df" in st.session_state:
                        del st.session_state.hits_df
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
                    if "hits_df" in st.session_state:
                        del st.session_state.hits_df
                    st.rerun()
