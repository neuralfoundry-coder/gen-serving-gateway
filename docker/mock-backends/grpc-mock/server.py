"""
gRPC Mock Backend Server for Image Generation
"""

import asyncio
import base64
import os
import random
import time
from concurrent import futures

import grpc
import imagebackend_pb2
import imagebackend_pb2_grpc

MOCK_NAME = os.getenv("MOCK_NAME", "grpc-mock-backend")
RESPONSE_DELAY_MIN = int(os.getenv("RESPONSE_DELAY_MIN", "50"))
RESPONSE_DELAY_MAX = int(os.getenv("RESPONSE_DELAY_MAX", "150"))


def generate_mock_image(width: int, height: int, seed: int = None) -> str:
    """Generate a mock image as base64 string"""
    if seed:
        random.seed(seed)
    
    r = random.randint(50, 200)
    g = random.randint(50, 200)
    b = random.randint(50, 200)
    
    header = f"P6\n{width} {height}\n255\n".encode()
    pixels = bytes([r, g, b] * (width * height))
    
    image_data = header + pixels
    return base64.b64encode(image_data).decode()


class ImageBackendServicer(imagebackend_pb2_grpc.ImageBackendServiceServicer):
    """gRPC Image Backend Service Implementation"""
    
    def Generate(self, request, context):
        """Generate images from a text prompt"""
        # Simulate processing delay
        delay = random.randint(RESPONSE_DELAY_MIN, RESPONSE_DELAY_MAX) / 1000.0
        time.sleep(delay)
        
        # Generate mock images
        images = []
        for i in range(request.n if request.n > 0 else 1):
            seed = request.seed + i if request.seed > 0 else random.randint(0, 2**32)
            
            image_data = imagebackend_pb2.ImageData(
                b64_json=generate_mock_image(
                    request.width if request.width > 0 else 1024,
                    request.height if request.height > 0 else 1024,
                    seed
                ),
                revised_prompt=f"[{MOCK_NAME}] {request.prompt}",
                seed=seed
            )
            images.append(image_data)
        
        return imagebackend_pb2.GenerateResponse(
            created=int(time.time()),
            data=images,
            model=request.model or "grpc-mock-v1"
        )
    
    def HealthCheck(self, request, context):
        """Health check endpoint"""
        return imagebackend_pb2.HealthCheckResponse(
            healthy=True,
            message=f"{MOCK_NAME} is healthy",
            available_models=["grpc-mock-v1", "grpc-mock-v2"]
        )


def serve():
    """Start the gRPC server"""
    server = grpc.server(futures.ThreadPoolExecutor(max_workers=10))
    imagebackend_pb2_grpc.add_ImageBackendServiceServicer_to_server(
        ImageBackendServicer(), server
    )
    
    port = 50051
    server.add_insecure_port(f"[::]:{port}")
    server.start()
    
    print(f"gRPC Mock Backend '{MOCK_NAME}' started on port {port}")
    
    try:
        server.wait_for_termination()
    except KeyboardInterrupt:
        server.stop(0)


if __name__ == "__main__":
    serve()

