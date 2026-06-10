export const TYPE_LABEL: Record<string, string> = {
  jab: "Jab",
  cross: "Cross",
  hook: "Gancho",
  uppercut: "Upper",
};

export const ARM_LABEL: Record<string, string> = {
  derecha: "Derecha",
  izquierda: "Izquierda",
};

export const HEIGHT_LABEL: Record<string, string> = {
  arriba: "Cabeza",
  abajo: "Cuerpo",
};

export const ARM_ICON: Record<string, string> = {
  derecha: "↗",
  izquierda: "↖",
};

export const HEIGHT_ICON: Record<string, string> = {
  arriba: "↑",
  abajo: "↓",
};

const TYPE_FROM: Record<string, string> = {
  jab: "jab",
  cross: "cross",
  hook: "hook",
  uppercut: "uppercut",
  gancho: "hook",
  upper: "uppercut",
};
const LIMB_FROM: Record<string, string> = {
  derecha: "derecha",
  izquierda: "izquierda",
};
const POS_FROM: Record<string, string> = {
  arriba: "arriba",
  abajo: "abajo",
  cabeza: "arriba",
  cuerpo: "abajo",
};

export function normType(ev: { class_name: string }): string {
  const raw = (ev.class_name || "").toLowerCase().split("_")[0];
  return TYPE_FROM[raw] ?? raw;
}

export function normLimb(ev: { limb: string }): string {
  const raw = (ev.limb || "").toLowerCase();
  return LIMB_FROM[raw] ?? raw;
}

export function normPos(ev: { position: string }): string {
  const raw = (ev.position || "").toLowerCase();
  return POS_FROM[raw] ?? raw;
}

export const RECENT_MAX = 15;
export const PUNCH_COOLDOWN_MS = 800;
export const IDLE_MS = 15000;
export const SENSOR_ONLINE_S = 3;
export const SENSOR_STALE_S = 15;
export const SENSORS_POLL_MS = 2000;

export const MODE_TRAINING_TYPE: Record<string, string> = {
  eval: "Evaluation",
  force: "ForceMeter",
  guided: "GuidedTraining",
};

export const ACCEL_SENSOR_SCALE = 1000;
export const FORCE_SCORE_MAX = 999;
export const FORCE_GRAVITY_G = 1.0;

export const FORCE_DEFAULTS = {
  minG: 0.4,
  maxG: 2.5,
  releaseG: 0.25,
  holdMs: 5000,
  autoMax: false,
};

export const FORCE_CFG_KEY = "ks.forceCfg.v2";

export const FORCE_LEVELS = [
  { min: 0, label: "Débil", color: "#8a93a6" },
  { min: 20, label: "Flojo", color: "#4ab8ff" },
  { min: 40, label: "Medio", color: "#3ddc97" },
  { min: 60, label: "Fuerte", color: "#f6c453" },
  { min: 80, label: "Potente", color: "#ff8a3d" },
  { min: 92, label: "¡Demoledor!", color: "#ff5d4a" },
];

export const GUIDED_TYPES = ["jab", "cross", "hook", "uppercut"];
export const GUIDED_LIMBS = ["derecha", "izquierda"];
export const GUIDED_POS = ["arriba", "abajo"];
export const GUIDED_NEXT_MS = 900;
