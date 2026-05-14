import os
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt
import seaborn as sns
import torch
import torch.nn as nn
from torch.utils.data import DataLoader, TensorDataset
from sklearn.model_selection import train_test_split
from sklearn.preprocessing import LabelEncoder
from sklearn.metrics import classification_report, confusion_matrix

from pipeline import DEFAULT_DATASET, FEATURE_COLS, WINDOW_SIZE, load_dataset
from model_def import PunchCNN

MODEL_DIR = os.path.join(os.path.dirname(os.path.abspath(__file__)), "model")
MODEL_PATH = os.path.join(MODEL_DIR, "punch_classifier.pt")
CLASS_NAMES_PATH = os.path.join(MODEL_DIR, "class_names.npy")
NORM_MEAN_PATH = os.path.join(MODEL_DIR, "norm_mean.npy")
NORM_STD_PATH = os.path.join(MODEL_DIR, "norm_std.npy")

DEVICE = torch.device("cuda" if torch.cuda.is_available() else "cpu")


def normalize(X: np.ndarray) -> tuple[np.ndarray, np.ndarray, np.ndarray]:
    mean = X.mean(axis=(0, 1), keepdims=True)
    std = X.std(axis=(0, 1), keepdims=True) + 1e-8
    return (X - mean) / std, mean, std


def augment(
    X: np.ndarray, y: np.ndarray, factor: int = 2
) -> tuple[np.ndarray, np.ndarray]:
    rng = np.random.default_rng(42)
    parts_X, parts_y = [X], [y]
    for _ in range(factor):
        noise = rng.normal(0, 0.05, X.shape).astype(np.float32)
        parts_X.append(X + noise)
        parts_y.append(y)
    return np.concatenate(parts_X), np.concatenate(parts_y)


def train_epoch(model, loader, criterion, optimizer):
    model.train()
    total_loss, correct = 0.0, 0
    for X_batch, y_batch in loader:
        X_batch, y_batch = X_batch.to(DEVICE), y_batch.to(DEVICE)
        optimizer.zero_grad()
        logits = model(X_batch)
        loss = criterion(logits, y_batch)
        loss.backward()
        optimizer.step()
        total_loss += loss.item() * len(y_batch)
        correct += (logits.argmax(1) == y_batch).sum().item()
    n = len(loader.dataset)
    return total_loss / n, correct / n


@torch.no_grad()
def eval_epoch(model, loader, criterion):
    model.eval()
    total_loss, correct = 0.0, 0
    for X_batch, y_batch in loader:
        X_batch, y_batch = X_batch.to(DEVICE), y_batch.to(DEVICE)
        logits = model(X_batch)
        total_loss += criterion(logits, y_batch).item() * len(y_batch)
        correct += (logits.argmax(1) == y_batch).sum().item()
    n = len(loader.dataset)
    return total_loss / n, correct / n


def plot_history(train_acc, val_acc, train_loss, val_loss, out_path: str) -> None:
    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(12, 4))
    ax1.plot(train_acc, label="train")
    ax1.plot(val_acc, label="val")
    ax1.set_title("Accuracy")
    ax1.set_xlabel("Epoch")
    ax1.legend()
    ax2.plot(train_loss, label="train")
    ax2.plot(val_loss, label="val")
    ax2.set_title("Loss")
    ax2.set_xlabel("Epoch")
    ax2.legend()
    plt.tight_layout()
    plt.savefig(out_path, dpi=120)
    plt.close()
    print(f"Historial guardado en {out_path}")


def plot_confusion_matrix(y_true, y_pred, class_names, out_path: str) -> None:
    cm = confusion_matrix(y_true, y_pred, labels=class_names)
    size = max(6, len(class_names))
    fig, ax = plt.subplots(figsize=(size, size - 1))
    sns.heatmap(
        cm,
        annot=True,
        fmt="d",
        cmap="Blues",
        xticklabels=class_names,
        yticklabels=class_names,
        ax=ax,
    )
    ax.set_xlabel("Predicho")
    ax.set_ylabel("Real")
    ax.set_title("Matriz de confusión")
    plt.xticks(rotation=45, ha="right")
    plt.tight_layout()
    plt.savefig(out_path, dpi=120)
    plt.close()
    print(f"Matriz de confusión guardada en {out_path}")


def main() -> None:
    print(f"Usando dispositivo: {DEVICE}\n")

    # ---- Load ----
    X, y_str, _ids = load_dataset(DEFAULT_DATASET)

    if len(y_str) == 0:
        print("ERROR: el dataset está vacío. Graba muestras con la app primero.")
        return

    print(f"Dataset cargado: {len(y_str)} muestras | shape X={X.shape}")

    counts = pd.Series(y_str).value_counts()
    print("\nDistribución de clases:")
    print(counts.to_string())
    print()

    if counts.min() < 10:
        print(
            f"ADVERTENCIA: '{counts.idxmin()}' solo tiene {counts.min()} muestras. "
            "Se recomienda al menos 30-50 por clase para resultados fiables.\n"
        )

    # ---- Encode labels ----
    le = LabelEncoder()
    y = le.fit_transform(y_str).astype(np.int64)
    class_names = le.classes_
    num_classes = len(class_names)
    print(f"Clases ({num_classes}): {class_names.tolist()}\n")

    # ---- Normalize ----
    X, mean, std = normalize(X)

    # ---- Train/val split ----
    X_train, X_val, y_train, y_val = train_test_split(
        X,
        y,
        test_size=0.2,
        random_state=42,
        stratify=y,
    )

    # ---- Augmentation on train only ----
    X_train, y_train = augment(X_train, y_train, factor=2)
    print(f"Entrenamiento: {len(X_train)} muestras (con augmentation)")
    print(f"Validación:    {len(X_val)} muestras\n")

    # ---- DataLoaders ----
    train_ds = TensorDataset(
        torch.from_numpy(X_train).float(),
        torch.from_numpy(y_train).long(),
    )
    val_ds = TensorDataset(
        torch.from_numpy(X_val).float(),
        torch.from_numpy(y_val).long(),
    )
    train_loader = DataLoader(train_ds, batch_size=32, shuffle=True)
    val_loader = DataLoader(val_ds, batch_size=64)

    # ---- Build model ----
    model = PunchCNN(num_classes).to(DEVICE)
    print(model)
    total_params = sum(p.numel() for p in model.parameters() if p.requires_grad)
    print(f"Parámetros entrenables: {total_params:,}\n")

    criterion = nn.CrossEntropyLoss()
    optimizer = torch.optim.Adam(model.parameters(), lr=1e-3)
    scheduler = torch.optim.lr_scheduler.ReduceLROnPlateau(
        optimizer,
        mode="min",
        factor=0.5,
        patience=5,
    )

    # ---- Training loop with early stopping ----
    best_val_acc = 0.0
    patience_counter = 0
    patience = 30
    best_state = None

    train_accs, val_accs, train_losses, val_losses = [], [], [], []

    for epoch in range(1, 81):
        tr_loss, tr_acc = train_epoch(model, train_loader, criterion, optimizer)
        vl_loss, vl_acc = eval_epoch(model, val_loader, criterion)
        scheduler.step(vl_loss)

        train_accs.append(tr_acc)
        val_accs.append(vl_acc)
        train_losses.append(tr_loss)
        val_losses.append(vl_loss)

        print(
            f"Epoch {epoch:3d}/80 | "
            f"loss {tr_loss:.4f} acc {tr_acc * 100:.1f}% | "
            f"val_loss {vl_loss:.4f} val_acc {vl_acc * 100:.1f}%"
        )

        if vl_acc > best_val_acc:
            best_val_acc = vl_acc
            best_state = {k: v.cpu().clone() for k, v in model.state_dict().items()}
            patience_counter = 0
        else:
            patience_counter += 1
            if patience_counter >= patience:
                print(f"\nEarly stopping en epoch {epoch}")
                break

    model.load_state_dict(best_state)

    # ---- Evaluate on val set ----
    print(f"\nMejor val_accuracy: {best_val_acc * 100:.1f}%")

    model.eval()
    all_preds = []
    with torch.no_grad():
        for X_batch, _ in val_loader:
            logits = model(X_batch.to(DEVICE))
            all_preds.append(logits.argmax(1).cpu().numpy())

    y_pred_idx = np.concatenate(all_preds)
    y_pred_str = le.inverse_transform(y_pred_idx)
    y_val_str = le.inverse_transform(y_val)

    print("\nReporte de clasificación:")
    print(classification_report(y_val_str, y_pred_str, target_names=class_names))

    # ---- Save ----
    os.makedirs(MODEL_DIR, exist_ok=True)
    torch.save(
        {
            "model_state": best_state,
            "num_classes": num_classes,
            "in_channels": len(FEATURE_COLS),
            "class_names": class_names.tolist(),
        },
        MODEL_PATH,
    )
    np.save(CLASS_NAMES_PATH, class_names)
    np.save(NORM_MEAN_PATH, mean)
    np.save(NORM_STD_PATH, std)

    print(f"\nModelo guardado en  {MODEL_PATH}")
    print(f"Clases en           {CLASS_NAMES_PATH}")
    print(f"Normalización en    {NORM_MEAN_PATH} / {NORM_STD_PATH}")

    # ---- Plots ----
    plot_history(
        train_accs,
        val_accs,
        train_losses,
        val_losses,
        os.path.join(MODEL_DIR, "training_history.png"),
    )
    plot_confusion_matrix(
        y_val_str,
        y_pred_str,
        class_names,
        os.path.join(MODEL_DIR, "confusion_matrix.png"),
    )


if __name__ == "__main__":
    main()
