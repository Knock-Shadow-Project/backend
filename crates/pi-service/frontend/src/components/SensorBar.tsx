import { useEffect, useState } from "react";
import type { SensorInfo } from "../types";
import { SENSOR_ONLINE_S, SENSOR_STALE_S } from "../constants";
import { sensorState, fmtSecondsAgo, macSuffix } from "../utils";

interface Props {
  sensors: SensorInfo[];
}

function computeSecondsAgo(s: SensorInfo): number | null {
  if (s.last_seen) {
    const t = Date.parse(s.last_seen);
    if (Number.isFinite(t)) return (Date.now() - t) / 1000;
  }
  return s.seconds_ago ?? null;
}

export function SensorBar({ sensors }: Props) {
  const [, tick] = useState(0);

  useEffect(() => {
    const id = setInterval(() => tick((n) => n + 1), 1000);
    return () => clearInterval(id);
  }, []);

  if (sensors.length === 0) {
    return (
      <section id="sensors-bar" aria-label="sensor status">
        <span className="sensors-label">Sensores</span>
        <span
          className="sensor-pill"
          data-state="unknown"
          title="DEVICE_MAC no configurado"
        >
          <span className="dot" />
          Ninguno configurado
        </span>
      </section>
    );
  }

  return (
    <section id="sensors-bar" aria-label="sensor status">
      <span className="sensors-label">Sensores</span>
      {sensors.map((s) => {
        const secs = computeSecondsAgo(s);
        const st = sensorState(secs, SENSOR_ONLINE_S, SENSOR_STALE_S);
        const title = `${s.mac}${s.device_name ? ` (${s.device_name})` : ""}`;
        return (
          <span
            key={s.mac}
            className="sensor-pill"
            data-state={st}
            title={title}
          >
            <span className="dot" />
            <span>Sensor {s.index}</span>
            <span className="mac">{macSuffix(s.mac)}</span>
            <span className="age">· {fmtSecondsAgo(secs)}</span>
          </span>
        );
      })}
    </section>
  );
}
