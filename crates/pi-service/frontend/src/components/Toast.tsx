import { useEffect, useState } from "react";

export interface ToastMessage {
  text: string;
  isError?: boolean;
  id: number;
}

interface Props {
  message: ToastMessage | null;
}

export function Toast({ message }: Props) {
  const [visible, setVisible] = useState(false);
  const [current, setCurrent] = useState<ToastMessage | null>(null);

  useEffect(() => {
    if (!message) return;
    setCurrent(message);
    setVisible(true);
    const id = setTimeout(() => setVisible(false), 3200);
    return () => clearTimeout(id);
  }, [message]);

  return (
    <div
      id="toast"
      role="status"
      aria-live="polite"
      className={`${visible ? "show" : ""}${current?.isError ? " error" : ""}`}
    >
      {current?.text ?? ""}
    </div>
  );
}
