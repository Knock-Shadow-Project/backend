import { useCallback, useEffect, useRef, useState } from "react";
import type { PunchEvent, SensorInfo, ActiveTraining, ConnState, Mode } from "./types";
import { RECENT_MAX, SENSORS_POLL_MS, MODE_TRAINING_TYPE, PUNCH_COOLDOWN_MS } from "./constants";
import { Header } from "./components/Header";
import { SensorBar } from "./components/SensorBar";
import { TrainingBar } from "./components/TrainingBar";
import { EvalPanel } from "./components/EvalPanel";
import { ForcePanel } from "./components/ForcePanel";
import { GuidedPanel } from "./components/GuidedPanel";
import { Toast } from "./components/Toast";
import type { ToastMessage } from "./components/Toast";

let toastCounter = 0;

export function App() {
  const [connState, setConnState] = useState<ConnState>("connecting");
  const [recent, setRecent] = useState<PunchEvent[]>([]);
  const [sensors, setSensors] = useState<SensorInfo[]>([]);
  const [activeTraining, setActiveTraining] = useState<ActiveTraining | null>(null);
  const [mode, setMode] = useState<Mode>("eval");
  const [toast, setToast] = useState<ToastMessage | null>(null);
  const [evalFlashKey, setEvalFlashKey] = useState(0);
  const [latestPunch, setLatestPunch] = useState<PunchEvent | null>(null);

  const wsRef = useRef<WebSocket | null>(null);
  const backoffRef = useRef(500);
  const reconnectTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const lastPunchTsRef = useRef(0);

  function showToast(text: string, isError = false) {
    setToast({ text, isError, id: ++toastCounter });
  }

  // ---- WebSocket ----
  const handlePunch = useCallback((ev: PunchEvent) => {
    const now = Date.now();
    if (now - lastPunchTsRef.current < PUNCH_COOLDOWN_MS) return;
    lastPunchTsRef.current = now;
    const ts = Date.parse(ev.detected_at) || now;
    const enriched: PunchEvent = { ...ev, _ts: ts };
    setRecent((prev) => [enriched, ...prev].slice(0, RECENT_MAX));
    setLatestPunch(enriched);
    setEvalFlashKey((k) => k + 1);
  }, []);

  function scheduleReconnect() {
    setConnState("disconnected");
    if (reconnectTimerRef.current) clearTimeout(reconnectTimerRef.current);
    const delay = backoffRef.current;
    reconnectTimerRef.current = setTimeout(connectWs, delay);
    backoffRef.current = Math.min(backoffRef.current * 2, 8000);
  }

  function connectWs() {
    if (reconnectTimerRef.current) clearTimeout(reconnectTimerRef.current);
    try {
      const proto = location.protocol === "https:" ? "wss:" : "ws:";
      const ws = new WebSocket(`${proto}//${location.host}/live`);
      wsRef.current = ws;
      setConnState("connecting");
      ws.addEventListener("open", () => {
        backoffRef.current = 500;
        setConnState("connected");
      });
      ws.addEventListener("message", (e) => {
        try {
          handlePunch(JSON.parse(e.data) as PunchEvent);
        } catch (_e) { /* ignore */ }
      });
      ws.addEventListener("close", scheduleReconnect);
      ws.addEventListener("error", () => {
        try { ws.close(); } catch (_e) { /* ignore */ }
      });
    } catch (_e) {
      scheduleReconnect();
    }
  }

  // ---- Training ----
  async function refreshActiveTraining() {
    try {
      const r = await fetch("/training/active", { cache: "no-store" });
      if (!r.ok) throw new Error(`HTTP ${r.status}`);
      const data = await r.json() as {
        active: boolean;
        local_training_id?: number;
        user_id?: number;
        start_time?: string;
      };
      setActiveTraining(
        data.active
          ? {
              local_training_id: data.local_training_id!,
              user_id: data.user_id!,
              start_time: data.start_time!,
            }
          : null,
      );
    } catch (e) {
      console.warn("refreshActiveTraining:", e);
    }
  }

  async function startTraining(userId: number) {
    try {
      const r = await fetch("/training/start", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          user_id: userId,
          training_type: MODE_TRAINING_TYPE[mode] ?? "Standard",
        }),
      });
      if (!r.ok) {
        const body = await r.text();
        throw new Error(body || `HTTP ${r.status}`);
      }
      showToast("training started");
      await refreshActiveTraining();
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : String(e);
      showToast(`failed to start: ${msg}`, true);
      throw e;
    }
  }

  async function stopTraining() {
    if (!activeTraining) return;
    try {
      const r = await fetch("/training/stop", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          local_training_id: activeTraining.local_training_id,
        }),
      });
      if (!r.ok) {
        const body = await r.text();
        throw new Error(body || `HTTP ${r.status}`);
      }
      showToast("training stopped");
      await refreshActiveTraining();
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : String(e);
      showToast(`failed to stop: ${msg}`, true);
      throw e;
    }
  }

  // ---- Sensors ----
  async function refreshSensors() {
    try {
      const r = await fetch("/sensors", { cache: "no-store" });
      if (!r.ok) throw new Error(`HTTP ${r.status}`);
      const data = await r.json() as SensorInfo[];
      setSensors(data);
    } catch (e) {
      console.warn("refreshSensors:", e);
    }
  }

  // ---- Pre-fill timeline ----
  async function prefillTimeline(training: ActiveTraining) {
    try {
      const r = await fetch(`/trainings/${training.local_training_id}/punches`, {
        cache: "no-store",
      });
      if (!r.ok) return;
      const punches = await r.json() as PunchEvent[];
      if (!Array.isArray(punches) || punches.length === 0) return;
      const sorted = punches
        .slice()
        .sort(
          (a, b) =>
            (Date.parse(b.detected_at) || 0) - (Date.parse(a.detected_at) || 0),
        )
        .slice(0, RECENT_MAX)
        .map((p) => ({ ...p, _ts: Date.parse(p.detected_at) || Date.now() }));
      setRecent(sorted);
      setLatestPunch(sorted[0] ?? null);
    } catch (e) {
      console.warn("prefillTimeline:", e);
    }
  }

  // ---- Boot ----
  useEffect(() => {
    (async () => {
      await refreshActiveTraining();
    })();

    const sensorsInterval = setInterval(refreshSensors, SENSORS_POLL_MS);
    refreshSensors();
    connectWs();

    return () => {
      clearInterval(sensorsInterval);
      if (reconnectTimerRef.current) clearTimeout(reconnectTimerRef.current);
      wsRef.current?.close();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    if (activeTraining) prefillTimeline(activeTraining);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [activeTraining?.local_training_id]);

  return (
    <>
      <Header connState={connState} />
      <SensorBar sensors={sensors} />
      <TrainingBar
        activeTraining={activeTraining}
        mode={mode}
        onStart={startTraining}
        onStop={stopTraining}
      />
      <nav id="mode-tabs" role="tablist">
        {(["eval", "force", "guided"] as Mode[]).map((m) => (
          <button
            key={m}
            className="mode-tab"
            role="tab"
            data-mode={m}
            aria-selected={mode === m ? "true" : "false"}
            onClick={() => setMode(m)}
          >
            {m === "eval"
              ? "Evaluación"
              : m === "force"
                ? "Medidor de fuerza"
                : "Entrenamiento guiado"}
          </button>
        ))}
      </nav>
      <main>
        {mode === "eval" && (
          <EvalPanel
            recent={recent}
            activeTraining={activeTraining}
            flashKey={evalFlashKey}
          />
        )}
        {mode === "force" && <ForcePanel active={mode === "force"} />}
        {mode === "guided" && (
          <GuidedPanel latestPunch={latestPunch} active={mode === "guided"} />
        )}
      </main>
      <Toast message={toast} />
    </>
  );
}
