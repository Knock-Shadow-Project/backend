import torch
import torch.nn as nn

from pipeline import FEATURE_COLS


class PunchCNN(nn.Module):
    def __init__(self, num_classes: int, in_channels: int = len(FEATURE_COLS)):
        super().__init__()
        self.features = nn.Sequential(
            nn.Conv1d(in_channels, 32, kernel_size=5, padding=2),
            nn.BatchNorm1d(32),
            nn.ReLU(),
            nn.MaxPool1d(2),
            nn.Conv1d(32, 64, kernel_size=5, padding=2),
            nn.BatchNorm1d(64),
            nn.ReLU(),
            nn.MaxPool1d(2),
            nn.Conv1d(64, 128, kernel_size=3, padding=1),
            nn.ReLU(),
        )
        self.classifier = nn.Sequential(
            nn.Linear(128, 64),
            nn.ReLU(),
            nn.Dropout(0.3),
            nn.Linear(64, num_classes),
        )

    def forward(self, x: torch.Tensor) -> torch.Tensor:
        # Input: (batch, seq_len, channels) → Conv1d needs (batch, channels, seq_len)
        x = x.transpose(1, 2)
        x = self.features(x)
        x = x.mean(dim=-1)  # global average pooling
        return self.classifier(x)
