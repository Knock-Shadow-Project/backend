import datetime
import numpy as np
import pandas as pd
import streamlit as st
import plotly.graph_objects as go
from plotly.subplots import make_subplots

from pipeline import (
    DEFAULT_DATASET,
    FEATURE_COLS,
    HIT_THRESHOLD_G,
    SENSOR_MAC_1,
    SENSOR_MAC_2,
    create_windows,
    detect_hits,
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
    X, y = load_dataset(DEFAULT_DATASET)
    if len(y) > 0:
        st.metric("Total muestras", len(y))
        counts = pd.Series(y).value_counts().rename("muestras")
        st.dataframe(counts, use_container_width=True)
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
    # If threshold changed and we have processed data, invalidate peaks
    if new_threshold != st.session_state.last_threshold:
        st.session_state.threshold = new_threshold
        st.session_state.last_threshold = new_threshold
        if (
            st.session_state.merged_df is not None
            and not st.session_state.merged_df.empty
        ):
            peaks = detect_hits(st.session_state.merged_df, threshold=new_threshold)
            st.session_state.peaks = peaks
            st.session_state.windows = create_windows(st.session_state.merged_df, peaks)

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
            # Reset processed data so it gets reloaded
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

    # Labeling — only shown after a valid recording
    if not st.session_state.recording and st.session_state.end_time:
        st.divider()
        st.subheader("Etiquetado")

        punch_type = st.selectbox("Tipo de golpe", PUNCH_TYPES)
        position = st.selectbox("Posición", POSITIONS)
        label = f"{punch_type}_{position}"
        st.info(f"Etiqueta: **{label}**")

        n_windows = (
            len(st.session_state.windows) if st.session_state.windows is not None else 0
        )
        st.metric("Ventanas detectadas", n_windows)

        if st.button(
            "Guardar en dataset",
            type="primary",
            use_container_width=True,
            disabled=(n_windows == 0),
        ):
            y_labels = np.array([label] * n_windows)
            total = save_dataset(st.session_state.windows, y_labels)
            st.success(f"Guardadas {n_windows} muestras — total en dataset: {total}")
            # Reset for next recording
            st.session_state.end_time = None
            st.session_state.merged_df = None
            st.session_state.peaks = None
            st.session_state.windows = None
            st.rerun()

# ---- Visualization ----
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
                    windows = create_windows(merged, peaks)
                    st.session_state.merged_df = merged
                    st.session_state.peaks = peaks
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
                        mode="markers",
                        name="golpes",
                        marker=dict(color="red", size=10, symbol="x"),
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
