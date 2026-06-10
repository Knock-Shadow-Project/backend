import { useEffect, useState } from "react";
import type { ActiveTraining, Mode } from "../types";
import { fmtDuration } from "../utils";

interface Props {
  activeTraining: ActiveTraining | null;
  mode: Mode;
  onStart: (userId: number) => Promise<void>;
  onStop: () => Promise<void>;
}

export function TrainingBar({ activeTraining, onStart, onStop }: Props) {
  const [userId, setUserId] = useState(1);
  const [starting, setStarting] = useState(false);
  const [stopping, setStopping] = useState(false);
  const [, tick] = useState(0);

  useEffect(() => {
    if (!activeTraining) return;
    const id = setInterval(() => tick((n) => n + 1), 1000);
    return () => clearInterval(id);
  }, [activeTraining]);

  async function handleStart() {
    setStarting(true);
    try {
      await onStart(userId);
    } finally {
      setStarting(false);
    }
  }

  async function handleStop() {
    setStopping(true);
    try {
      await onStop();
    } finally {
      setStopping(false);
    }
  }

  return (
    <section id="training-bar">
      <div id="training-info" className={activeTraining ? "active" : ""}>
        {activeTraining ? (
          <>
            <span className="badge">
              Entrenamiento #{activeTraining.local_training_id}
            </span>
            user {activeTraining.user_id} ·{" "}
            {fmtDuration(activeTraining.start_time)}
          </>
        ) : (
          "entrenamiento inactivo"
        )}
      </div>
      <input
        id="user-id-input"
        type="number"
        value={activeTraining ? activeTraining.user_id : userId}
        min={1}
        aria-label="user id"
        disabled={!!activeTraining}
        onChange={(e) => setUserId(Number(e.target.value))}
      />
      <button
        className="primary"
        disabled={!!activeTraining || starting}
        onClick={handleStart}
      >
        Iniciar
      </button>
      <button
        className="danger"
        disabled={!activeTraining || stopping}
        onClick={handleStop}
      >
        Parar
      </button>
    </section>
  );
}
