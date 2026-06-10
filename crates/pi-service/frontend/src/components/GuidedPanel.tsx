import { useState, useCallback, useEffect } from "react";
import type { PunchEvent, GuidedState, GuidedTarget } from "../types";
import {
  TYPE_LABEL,
  ARM_LABEL,
  HEIGHT_LABEL,
  GUIDED_TYPES,
  GUIDED_LIMBS,
  GUIDED_POS,
  GUIDED_NEXT_MS,
  normType,
  normLimb,
  normPos,
} from "../constants";
import { describe } from "../utils";

interface Props {
  latestPunch: PunchEvent | null;
  active: boolean;
}

function cap(s: string) {
  return s && s.length ? s[0].toUpperCase() + s.slice(1) : s || "?";
}

function randomTarget(prev: GuidedTarget | null): GuidedTarget {
  let t: GuidedTarget;
  do {
    t = {
      type: GUIDED_TYPES[Math.floor(Math.random() * GUIDED_TYPES.length)]!,
      limb: GUIDED_LIMBS[Math.floor(Math.random() * GUIDED_LIMBS.length)]!,
      position: GUIDED_POS[Math.floor(Math.random() * GUIDED_POS.length)]!,
    };
  } while (
    prev &&
    t.type === prev.type &&
    t.limb === prev.limb &&
    t.position === prev.position
  );
  return t;
}

function matchesTarget(
  ev: PunchEvent,
  t: GuidedTarget,
  strictness: string,
): boolean {
  if (normType(ev) !== t.type) return false;
  if (strictness === "type") return true;
  if (normLimb(ev) !== t.limb) return false;
  if (strictness === "type_limb") return true;
  return normPos(ev) === t.position;
}

function initGuided(goal: number): GuidedState {
  return {
    target: randomTarget(null),
    hits: 0,
    miss: 0,
    count: 0,
    goal,
    finished: false,
  };
}

export function GuidedPanel({ latestPunch, active }: Props) {
  const [goal, setGoal] = useState(10);
  const [strictness, setStrictness] = useState("full");
  const [override, setOverride] = useState(false);
  const [guided, setGuided] = useState<GuidedState>(() => initGuided(10));
  const [feedback, setFeedback] = useState<{
    text: string;
    kind: "ok" | "bad" | "";
  }>({ text: "", kind: "" });
  const [flashKind, setFlashKind] = useState<"ok" | "bad" | "">("");
  const [processedTs, setProcessedTs] = useState<number | null>(null);

  function nextTarget(g: GuidedState): GuidedState {
    return { ...g, target: randomTarget(g.target) };
  }

  const handlePunch = useCallback(
    (ev: PunchEvent) => {
      if (!active) return;
      setGuided((g) => {
        if (!g || g.finished) return g;
        const ok = override || matchesTarget(ev, g.target, strictness);
        const d = describe(ev);
        const newG = { ...g, count: g.count + 1 };
        if (ok) {
          newG.hits = g.hits + 1;
          setFeedback({
            text: override
              ? `✔ Validado (${d.type} ${d.arm} ${d.height})`
              : "✔ ¡Correcto!",
            kind: "ok",
          });
          setFlashKind("ok");
        } else {
          newG.miss = g.miss + 1;
          setFeedback({
            text: `✗ Fallo — detectado ${d.type} ${d.arm} ${d.height}`,
            kind: "bad",
          });
          setFlashKind("bad");
        }
        setTimeout(() => setFlashKind(""), 600);

        if (newG.goal > 0 && newG.count >= newG.goal) {
          return { ...newG, finished: true };
        }
        setTimeout(() => {
          setGuided((prev) =>
            prev && !prev.finished ? nextTarget(prev) : prev,
          );
          setFeedback({ text: "", kind: "" });
        }, GUIDED_NEXT_MS);
        return newG;
      });
    },
    [active, override, strictness],
  );

  useEffect(() => {
    if (!latestPunch) return;
    if (latestPunch._ts === processedTs) return;
    setProcessedTs(latestPunch._ts);
    handlePunch(latestPunch);
  }, [latestPunch, handlePunch, processedTs]);

  useEffect(() => {
    function onKeyDown(e: KeyboardEvent) {
      if (e.ctrlKey && e.shiftKey && e.code === "KeyV") {
        e.preventDefault();
        setOverride((v) => !v);
      }
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);

  function restart() {
    setGuided(initGuided(goal));
    setFeedback({ text: "", kind: "" });
    setFlashKind("");
  }

  function skip() {
    setGuided((g) => (g && !g.finished ? nextTarget(g) : g));
    setFeedback({ text: "", kind: "" });
  }

  const t = guided.target;
  const total = guided.hits + guided.miss;
  const acc = total > 0 ? `${Math.round((guided.hits / total) * 100)}%` : "—";

  return (
    <section id="panel-guided" className="panel" role="tabpanel">
      <div
        id="guided-card"
        className={`guided-card${flashKind === "ok" ? " flash-ok" : flashKind === "bad" ? " flash-bad" : ""}`}
      >
        {override && (
          <span
            className="override-indicator"
            title="Validar cualquier golpe activo (Ctrl+Shift+V para desactivar)"
          >
            ●
          </span>
        )}
        <div className="guided-prompt-label">Pega este golpe</div>
        <div className="guided-target">
          {guided.finished ? (
            "¡Bien hecho! 🥊"
          ) : (
            <>
              {TYPE_LABEL[t.type] ?? cap(t.type)}
              <small>
                {ARM_LABEL[t.limb] ?? cap(t.limb)} ·{" "}
                {HEIGHT_LABEL[t.position] ?? cap(t.position)}
              </small>
            </>
          )}
        </div>
        <div
          className={`guided-feedback${feedback.kind ? ` ${feedback.kind}` : ""}`}
        >
          {feedback.text}
        </div>
        <dl className="guided-stats">
          <div>
            <dt>Aciertos</dt>
            <dd>{guided.hits}</dd>
          </div>
          <div>
            <dt>Fallos</dt>
            <dd>{guided.miss}</dd>
          </div>
          <div>
            <dt>Precisión</dt>
            <dd>{acc}</dd>
          </div>
          <div>
            <dt>Progreso</dt>
            <dd>
              {guided.goal > 0 ? `${guided.count}/${guided.goal}` : `${guided.count}/∞`}
            </dd>
          </div>
        </dl>
        {guided.finished && (
          <div className="guided-summary">
            ✅ Sesión completada — <strong>{guided.hits}</strong> aciertos /{" "}
            <strong>{guided.miss}</strong> fallos · precisión{" "}
            <strong>{acc}</strong>
          </div>
        )}
        <div className="guided-controls">
          <label>
            Exigencia
            <select
              value={strictness}
              onChange={(e) => setStrictness(e.target.value)}
            >
              <option value="full">Completo (tipo + brazo + altura)</option>
              <option value="type_limb">Tipo + brazo</option>
              <option value="type">Solo tipo</option>
            </select>
          </label>
          <label>
            Objetivo
            <input
              type="number"
              min={0}
              value={goal}
              style={{ width: 64 }}
              onChange={(e) => {
                const n = Number(e.target.value);
                setGoal(n);
                if (!guided.finished)
                  setGuided((g) => ({ ...g, goal: n >= 0 ? n : 10 }));
              }}
            />
          </label>
          <button onClick={skip} disabled={guided.finished}>
            Saltar
          </button>
          <button className="primary" onClick={restart}>
            Reiniciar
          </button>
        </div>
      </div>
    </section>
  );
}
