import { useEffect, useRef, useState, useCallback } from "react";
import type { ForceCfg, ForceState } from "../types";
import {
  FORCE_DEFAULTS,
  FORCE_CFG_KEY,
  ACCEL_SENSOR_SCALE,
  FORCE_GRAVITY_G,
} from "../constants";
import { forceFractionFromG, forceLevelFor } from "../utils";

const G_TO_MS2 = 9.81;
const HIT_COOLDOWN_MS = 1000;
const HISTORY_MAX = 15;

interface PunchRecord {
  score: number;
  powerMs2: number;
  ts: number;
  seq: number; // monotonic insert order for stable sort
}

interface Props {
  active: boolean;
}

function loadCfg(): ForceCfg {
  try {
    const raw = localStorage.getItem(FORCE_CFG_KEY);
    if (raw) return { ...FORCE_DEFAULTS, ...JSON.parse(raw) };
  } catch (_e) {
    /* ignore */
  }
  return { ...FORCE_DEFAULTS };
}

function saveCfg(cfg: ForceCfg) {
  try {
    localStorage.setItem(FORCE_CFG_KEY, JSON.stringify(cfg));
  } catch (_e) {
    /* ignore */
  }
}

const initForce = (): ForceState => ({
  maxScore: 0,
  maxNet: 0,
  count: 0,
  inHit: false,
  hitPeak: 0,
  holdUntil: 0,
  cooldownUntil: 0,
});

export function ForcePanel({ active }: Props) {
  const [cfg, setCfg] = useState<ForceCfg>(loadCfg);
  const [force, setForce] = useState<ForceState>(initForce);
  const [mark, setMark] = useState<{
    netG: number;
    score: number;
    isRecord: boolean;
  } | null>(null);
  const [showMark, setShowMark] = useState(false);
  const [flashKey, setFlashKey] = useState(0);
  const [history, setHistory] = useState<PunchRecord[]>([]);
  const seqRef = useRef(0);

  const cfgRef = useRef(cfg);
  cfgRef.current = cfg;
  const forceRef = useRef(force);
  forceRef.current = force;

  const holdTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const wsRef = useRef<WebSocket | null>(null);
  const wsTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const registerHit = useCallback((gAbs: number) => {
    const c = cfgRef.current;
    const f = forceRef.current;
    const netG = Math.max(0, gAbs - FORCE_GRAVITY_G);

    const frac = forceFractionFromG(netG, c, f) ?? 0;
    const score = Math.round(frac * 999);
    const isRecord = score > f.maxScore;

    setForce((prev) => ({
      ...prev,
      count: prev.count + 1,
      maxScore: isRecord ? score : prev.maxScore,
      maxNet: netG > prev.maxNet ? netG : prev.maxNet,
      inHit: false,
      hitPeak: 0,
      holdUntil: Date.now() + c.holdMs,
      cooldownUntil: Date.now() + HIT_COOLDOWN_MS,
    }));
    setMark({ netG, score, isRecord });
    setShowMark(true);
    setFlashKey((k) => k + 1);
    const seq = ++seqRef.current;
    setHistory((prev) =>
      [
        { score, powerMs2: netG * G_TO_MS2, ts: Date.now(), seq },
        ...prev,
      ].slice(0, HISTORY_MAX),
    );

    if (holdTimerRef.current) clearTimeout(holdTimerRef.current);
    holdTimerRef.current = setTimeout(() => {
      setShowMark(false);
    }, c.holdMs + 50);
  }, []);

  const handleSample = useCallback(
    (sample: { x: number; y: number; z: number }) => {
      const c = cfgRef.current;
      const f = forceRef.current;

      // 1 s cooldown between punches
      if (Date.now() < f.cooldownUntil) return;

      const { x, y, z } = sample;
      if (![x, y, z].every(Number.isFinite)) return;
      const absG = Math.sqrt(x * x + y * y + z * z) / ACCEL_SENSOR_SCALE;
      const netG = Math.max(0, absG - FORCE_GRAVITY_G);

      if (!f.inHit) {
        if (netG >= c.minG) {
          setForce((prev) => ({ ...prev, inHit: true, hitPeak: absG }));
        }
      } else {
        if (absG > f.hitPeak) {
          setForce((prev) => ({ ...prev, hitPeak: absG }));
        }
        if (netG < c.releaseG) {
          registerHit(f.hitPeak);
        }
      }
    },
    [registerHit],
  );

  function connect() {
    if (wsRef.current) return;
    if (wsTimerRef.current) clearTimeout(wsTimerRef.current);
    try {
      const proto = location.protocol === "https:" ? "wss:" : "ws:";
      const ws = new WebSocket(`${proto}//${location.host}/live/accel`);
      wsRef.current = ws;
      ws.addEventListener("message", (e) => {
        try {
          handleSample(
            JSON.parse(e.data) as { x: number; y: number; z: number },
          );
        } catch (_e) {
          /* ignore */
        }
      });
      ws.addEventListener("close", () => {
        wsRef.current = null;
        if (active) {
          wsTimerRef.current = setTimeout(connect, 1000);
        }
      });
      ws.addEventListener("error", () => {
        try {
          ws.close();
        } catch (_e) {
          /* ignore */
        }
      });
    } catch (_e) {
      wsTimerRef.current = setTimeout(connect, 1000);
    }
  }

  function disconnect() {
    if (wsTimerRef.current) clearTimeout(wsTimerRef.current);
    const ws = wsRef.current;
    wsRef.current = null;
    if (ws) {
      try {
        ws.onclose = null;
        ws.close();
      } catch (_e) {
        /* ignore */
      }
    }
    setForce((prev) => ({ ...prev, inHit: false }));
  }

  useEffect(() => {
    if (active) connect();
    else disconnect();
    return () => disconnect();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [active]);

  function resetForce() {
    setForce(initForce());
    setMark(null);
    setShowMark(false);
    setHistory([]);
    if (holdTimerRef.current) clearTimeout(holdTimerRef.current);
  }

  const displayMark = showMark && mark != null;
  const frac = displayMark
    ? (forceFractionFromG(mark.netG, cfg, force) ?? 0)
    : 0;
  const pct = frac * 100;
  const lvl = forceLevelFor(pct);

  return (
    <section id="panel-force" className="panel" role="tabpanel">
      <div className="eval-grid">
        <ForceCard
          cfg={cfg}
          force={force}
          mark={displayMark ? mark : null}
          pct={pct}
          lvl={lvl}
          flashKey={flashKey}
          onReset={resetForce}
          onCfgChange={setCfg}
        />
        <ForceHistory history={history} />
      </div>
    </section>
  );
}

function ForceCard({
  force,
  mark,
  pct,
  lvl,
  flashKey,
  onReset,
  onCfgChange,
  cfg,
}: {
  cfg: ForceCfg;
  force: ForceState;
  mark: { netG: number; score: number; isRecord: boolean } | null;
  pct: number;
  lvl: { label: string; color: string };
  flashKey: number;
  onReset: () => void;
  onCfgChange: (c: ForceCfg) => void;
}) {
  const [flashing, setFlashing] = useState(false);

  useEffect(() => {
    if (flashKey === 0) return;
    setFlashing(true);
    const id = setTimeout(() => setFlashing(false), 600);
    return () => clearTimeout(id);
  }, [flashKey]);

  const powerMs2 = mark ? (mark.netG * G_TO_MS2).toFixed(1) : null;

  return (
    <div id="force-card" className={`force-card${flashing ? " flash" : ""}`}>
      <div className="lp-label guided-prompt-label">Medidor de fuerza</div>
      <div
        className="force-score"
        style={mark ? { color: lvl.color } : undefined}
      >
        {mark ? String(mark.score) : "—"}
      </div>
      <div
        className="force-level"
        style={mark ? { color: lvl.color } : undefined}
      >
        {mark ? lvl.label : "esperando golpe…"}
      </div>
      <div className="record-flash">
        {mark?.isRecord ? "🏆 ¡Nuevo récord!" : ""}
      </div>
      <div className="force-bar">
        <span style={{ width: mark ? `${pct}%` : "0%" }} />
      </div>
      <dl className="force-meta">
        <div>
          <dt>Potencia</dt>
          <dd>{powerMs2 != null ? `${powerMs2} m/s²` : "— m/s²"}</dd>
        </div>
        <div>
          <dt>Récord sesión</dt>
          <dd>{force.maxScore > 0 ? String(force.maxScore) : "—"}</dd>
        </div>
        <div>
          <dt>Golpes</dt>
          <dd>{force.count}</dd>
        </div>
      </dl>
      <button className="danger" style={{ marginTop: 18 }} onClick={onReset}>
        Reiniciar récord
      </button>
      <ForceSettings cfg={cfg} onApply={onCfgChange} />
    </div>
  );
}

function fmtAgo(ts: number): string {
  const s = Math.max(0, Math.round((Date.now() - ts) / 1000));
  if (s < 2) return "ahora";
  if (s < 60) return `${s}s`;
  return `${Math.floor(s / 60)}m`;
}

function ForceHistory({ history }: { history: PunchRecord[] }) {
  const [, tick] = useState(0);
  useEffect(() => {
    const id = setInterval(() => tick((n) => n + 1), 1000);
    return () => clearInterval(id);
  }, []);

  // Sort newest-first by seq (monotonic insert counter) to guarantee order
  const sorted = [...history].sort((a, b) => b.seq - a.seq);
  const bestScore =
    sorted.length > 0 ? Math.max(...sorted.map((h) => h.score)) : -1;

  return (
    <aside id="timeline">
      <h3>Historial</h3>
      <ul id="punch-list">
        {sorted.length === 0 ? (
          <li className="muted">sin golpes aún</li>
        ) : (
          sorted.map((h) => {
            const isBest = h.score === bestScore;
            return (
              <li key={h.seq}>
                <div
                  className="tl-type"
                  style={isBest ? { color: "var(--good)" } : undefined}
                >
                  {h.score} pts{isBest ? " 🏆" : ""}
                </div>
                <div className="tl-meta">{h.powerMs2.toFixed(1)} m/s²</div>
                <div className="tl-age">{fmtAgo(h.ts)}</div>
              </li>
            );
          })
        )}
      </ul>
    </aside>
  );
}

function ForceSettings({
  cfg,
  onApply,
}: {
  cfg: ForceCfg;
  onApply: (c: ForceCfg) => void;
}) {
  const [minG, setMinG] = useState(String(cfg.minG));
  const [maxG, setMaxG] = useState(String(cfg.maxG));
  const [releaseG, setReleaseG] = useState(String(cfg.releaseG));
  const [holdS, setHoldS] = useState(String(cfg.holdMs / 1000));
  const [autoMax, setAutoMax] = useState(cfg.autoMax);

  function apply() {
    const num = (s: string, def: number) => {
      const v = parseFloat(s);
      return Number.isFinite(v) ? v : def;
    };
    let newMinG = Math.max(0.05, num(minG, cfg.minG));
    let newMaxG = Math.max(newMinG + 0.1, num(maxG, cfg.maxG));
    let newRelease = Math.min(num(releaseG, cfg.releaseG), newMinG);
    newRelease = Math.max(0.02, newRelease);
    const newHoldMs = Math.max(0, num(holdS, cfg.holdMs / 1000)) * 1000;
    const newCfg: ForceCfg = {
      minG: newMinG,
      maxG: newMaxG,
      releaseG: newRelease,
      holdMs: newHoldMs,
      autoMax,
    };
    saveCfg(newCfg);
    onApply(newCfg);
    setMinG(String(newMinG));
    setMaxG(String(newMaxG));
    setReleaseG(String(newRelease));
    setHoldS(String(newHoldMs / 1000));
  }

  function reset() {
    const d = { ...FORCE_DEFAULTS };
    saveCfg(d);
    onApply(d);
    setMinG(String(d.minG));
    setMaxG(String(d.maxG));
    setReleaseG(String(d.releaseG));
    setHoldS(String(d.holdMs / 1000));
    setAutoMax(d.autoMax);
  }

  return (
    <details className="force-settings">
      <summary>Ajustes del medidor</summary>
      <div className="force-cfg-grid">
        <label>
          Umbral golpe (G neta)
          <input
            type="number"
            step="0.05"
            min="0.05"
            inputMode="decimal"
            value={minG}
            onChange={(e) => setMinG(e.target.value)}
          />
        </label>
        <label>
          Golpe máximo (G neta)
          <input
            type="number"
            step="0.1"
            min="0.1"
            inputMode="decimal"
            value={maxG}
            disabled={autoMax}
            onChange={(e) => setMaxG(e.target.value)}
          />
        </label>
        <label>
          Cierre golpe (G neta)
          <input
            type="number"
            step="0.05"
            min="0.02"
            inputMode="decimal"
            value={releaseG}
            onChange={(e) => setReleaseG(e.target.value)}
          />
        </label>
        <label>
          Mantener marca (s)
          <input
            type="number"
            step="0.5"
            min="0"
            inputMode="decimal"
            value={holdS}
            onChange={(e) => setHoldS(e.target.value)}
          />
        </label>
      </div>
      <label className="force-cfg-check">
        <input
          type="checkbox"
          checked={autoMax}
          onChange={(e) => setAutoMax(e.target.checked)}
        />
        Auto-calibrar tope al mejor golpe de la sesión
      </label>
      <div className="force-cfg-actions">
        <button className="primary" onClick={apply}>
          Aplicar
        </button>
        <button onClick={reset}>Por defecto</button>
      </div>
      <p className="force-cfg-hint">
        Puntuación 0–999 sobre la G <b>neta</b> (sin la gravedad de reposo). Se
        guarda en este navegador; sobrevive a recompilaciones.
      </p>
    </details>
  );
}
