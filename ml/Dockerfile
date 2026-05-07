# syntax=docker/dockerfile:1
FROM python:3.12-slim

WORKDIR /app

# Install system dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    gcc libpq-dev \
    && rm -rf /var/lib/apt/lists/*

# Install Python dependencies
COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

# Copy application code and model
COPY pipeline.py train.py api_client.py main.py app.py ./
COPY model/ ./model/
COPY data/ ./data/

# Default command (overridden in docker-compose)
CMD ["python", "main.py"]
