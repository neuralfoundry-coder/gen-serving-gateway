FROM python:3.11-slim

WORKDIR /app

RUN pip install --no-cache-dir grpcio grpcio-tools protobuf

COPY server.py .
COPY imagebackend_pb2.py .
COPY imagebackend_pb2_grpc.py .

EXPOSE 50051

CMD ["python", "server.py"]

