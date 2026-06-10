import type { ConnState } from "../types";

interface Props {
  connState: ConnState;
}

export function Header({ connState }: Props) {
  return (
    <header>
      <h1>
        Knock<span className="accent">Shadow</span> Pi
      </h1>
      <div id="conn-status" className="conn" data-state={connState}>
        <span className="dot" />
        <span className="label">
          {connState == "connected"
            ? "Conectado"
            : connState == "connecting"
              ? "Conectando"
              : "Desconectado"}
        </span>
      </div>
    </header>
  );
}
