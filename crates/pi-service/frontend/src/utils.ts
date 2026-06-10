import {
  TYPE_LABEL,
  ARM_LABEL,
  HEIGHT_LABEL,
  ARM_ICON,
  HEIGHT_ICON,
  normType,
  normLimb,
  normPos,
  FORCE_LEVELS,
  FORCE_SCORE_MAX,
  FORCE_GRAVITY_G,
} from "./constants";
import type { PunchDescription, ForceCfg, ForceState } from "./types";

function cap(s: string): string {
  return s && s.length ? s[0].toUpperCase() + s.slice(1) : s || "?";
}

export function describe(ev: {
  class_name: string;
  limb: string;
  position: string;
}): PunchDescription {
  const type = normType(ev);
  const limb = normLimb(ev);
  const pos = normPos(ev);
  return {
    type: TYPE_LABEL[type] ?? cap(type),
    arm: ARM_LABEL[limb] ?? cap(limb),
    height: HEIGHT_LABEL[pos] ?? cap(pos),
    armIcon: ARM_ICON[limb] ?? "",
    heightIcon: HEIGHT_ICON[pos] ?? "",
    armKey: limb,
  };
}

export function fmtAge(detectedAtMs: number): string {
  const diff = Math.max(0, Math.round((Date.now() - detectedAtMs) / 1000));
  if (diff < 2) return "ahora";
  if (diff < 60) return `hace ${diff}s`;
  const m = Math.floor(diff / 60);
  if (m < 60) return `hace ${m}m`;
  const h = Math.floor(m / 60);
  return `hace ${h}h`;
}

export function fmtSecondsAgo(secondsAgo: number | null | undefined): string {
  if (secondsAgo == null) return "nunca";
  const s = Math.max(0, Math.round(secondsAgo));
  if (s < 2) return "ahora";
  if (s < 60) return `hace ${s}s`;
  const m = Math.floor(s / 60);
  if (m < 60) return `hace ${m}m`;
  const h = Math.floor(m / 60);
  return `hace ${h}h`;
}

export function fmtDuration(startIso: string): string {
  const startMs = Date.parse(startIso);
  if (!Number.isFinite(startMs)) return "";
  const s = Math.max(0, Math.floor((Date.now() - startMs) / 1000));
  const mm = String(Math.floor(s / 60)).padStart(2, "0");
  const ss = String(s % 60).padStart(2, "0");
  return `${mm}:${ss}`;
}

export function sensorState(
  secondsAgo: number | null | undefined,
  onlineS: number,
  staleS: number,
): string {
  if (secondsAgo == null) return "desconocido";
  if (secondsAgo <= onlineS) return "activo";
  if (secondsAgo <= staleS) return "parado";
  return "inactivo";
}

export function macSuffix(mac: string): string {
  return mac && mac.length >= 5 ? mac.slice(-5) : mac || "?";
}

export function forceScaleTop(cfg: ForceCfg, force: ForceState): number {
  return cfg.autoMax ? Math.max(cfg.maxG, force.maxNet || 0) : cfg.maxG;
}

export function forceFractionFromG(
  netG: number,
  cfg: ForceCfg,
  force: ForceState,
): number | null {
  if (!Number.isFinite(netG)) return null;
  const minG = cfg.minG;
  const span = forceScaleTop(cfg, force) - minG || 1;
  return Math.max(0, Math.min(1, (netG - minG) / span));
}

export function forceLevelFor(pct: number) {
  let lvl = FORCE_LEVELS[0]!;
  for (const l of FORCE_LEVELS) if (pct >= l.min) lvl = l;
  return lvl;
}

export function netGFromAbs(absG: number): number {
  return Math.max(0, absG - FORCE_GRAVITY_G);
}

export function forceScore(
  netG: number,
  cfg: ForceCfg,
  force: ForceState,
): number {
  const frac = forceFractionFromG(netG, cfg, force) ?? 0;
  return Math.round(frac * FORCE_SCORE_MAX);
}
