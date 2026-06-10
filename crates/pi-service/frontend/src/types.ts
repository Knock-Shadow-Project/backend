export interface PunchEvent {
  class_name: string;
  limb: string;
  position: string;
  power?: number | null;
  prob: number;
  detected_at: string;
  _ts: number;
}

export interface SensorInfo {
  index: number;
  mac: string;
  device_name?: string;
  seconds_ago?: number | null;
  last_seen?: string | null;
}

export interface ActiveTraining {
  local_training_id: number;
  user_id: number;
  start_time: string;
}

export type ConnState = "connecting" | "connected" | "disconnected";

export type Mode = "eval" | "force" | "guided";

export interface ForceCfg {
  minG: number;
  maxG: number;
  releaseG: number;
  holdMs: number;
  autoMax: boolean;
}

export interface ForceState {
  maxScore: number;   // best score (0-999) this session
  maxNet: number;
  count: number;
  inHit: boolean;
  hitPeak: number;
  holdUntil: number;
  cooldownUntil: number; // ignore samples until this timestamp (ms)
}

export interface GuidedTarget {
  type: string;
  limb: string;
  position: string;
}

export interface GuidedState {
  target: GuidedTarget;
  hits: number;
  miss: number;
  count: number;
  goal: number;
  finished: boolean;
}

export interface PunchDescription {
  type: string;
  arm: string;
  height: string;
  armIcon: string;
  heightIcon: string;
  armKey: string;
}
