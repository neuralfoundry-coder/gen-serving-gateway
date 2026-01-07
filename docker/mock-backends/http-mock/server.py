"""
HTTP Mock Backend Server for Image Generation
Simulates various backend behaviors for testing
"""

import asyncio
import base64
import io
import os
import random
import time
import uuid
from datetime import datetime
from typing import Optional

from fastapi import FastAPI, HTTPException, Request
from fastapi.responses import JSONResponse
from pydantic import BaseModel

app = FastAPI(title="Mock Image Generation Backend")

# Configuration from environment
MOCK_NAME = os.getenv("MOCK_NAME", "mock-backend")
RESPONSE_DELAY_MIN = int(os.getenv("RESPONSE_DELAY_MIN", "50"))
RESPONSE_DELAY_MAX = int(os.getenv("RESPONSE_DELAY_MAX", "100"))
ERROR_RATE = int(os.getenv("ERROR_RATE", "0"))  # Percentage of requests that fail
TIMEOUT_RATE = int(os.getenv("TIMEOUT_RATE", "0"))  # Percentage of requests that timeout

# Metrics storage
metrics = {
    "requests_total": 0,
    "requests_success": 0,
    "requests_failed": 0,
    "avg_response_time": 0,
    "start_time": datetime.now().isoformat()
}


class GenerateRequest(BaseModel):
    prompt: str
    negative_prompt: Optional[str] = None
    n: int = 1
    width: int = 1024
    height: int = 1024
    model: Optional[str] = None
    seed: Optional[int] = None
    guidance_scale: Optional[float] = 7.5
    num_inference_steps: Optional[int] = 50
    response_format: str = "b64_json"


def generate_mock_image(width: int, height: int, seed: Optional[int] = None) -> str:
    """Generate a mock image as base64 string"""
    # Create a simple colored rectangle as mock image
    # In real testing, this could be a more sophisticated mock
    
    if seed:
        random.seed(seed)
    
    # Generate random RGB color
    r = random.randint(50, 200)
    g = random.randint(50, 200)
    b = random.randint(50, 200)
    
    # Create a simple PPM image (no PIL dependency needed for basic mock)
    # PPM format: P6\nwidth height\n255\n<binary RGB data>
    header = f"P6\n{width} {height}\n255\n".encode()
    pixels = bytes([r, g, b] * (width * height))
    
    # Encode to base64
    image_data = header + pixels
    return base64.b64encode(image_data).decode()


@app.get("/health")
async def health_check():
    """Health check endpoint"""
    return {
        "healthy": True,
        "name": MOCK_NAME,
        "timestamp": datetime.now().isoformat()
    }


@app.get("/metrics")
async def get_metrics():
    """Get server metrics"""
    return metrics


@app.post("/v1/images/generations")
async def generate_images(request: GenerateRequest):
    """OpenAI-compatible image generation endpoint"""
    start_time = time.time()
    metrics["requests_total"] += 1
    
    # Simulate timeout
    if TIMEOUT_RATE > 0 and random.randint(1, 100) <= TIMEOUT_RATE:
        # Sleep for a long time to simulate timeout
        await asyncio.sleep(120)
        return JSONResponse(
            status_code=504,
            content={"error": {"message": "Gateway timeout", "type": "timeout"}}
        )
    
    # Simulate errors
    if ERROR_RATE > 0 and random.randint(1, 100) <= ERROR_RATE:
        metrics["requests_failed"] += 1
        error_types = [
            (500, "Internal server error"),
            (503, "Service temporarily unavailable"),
            (429, "Rate limit exceeded"),
        ]
        status, message = random.choice(error_types)
        return JSONResponse(
            status_code=status,
            content={"error": {"message": message, "type": "server_error"}}
        )
    
    # Simulate processing delay
    delay = random.randint(RESPONSE_DELAY_MIN, RESPONSE_DELAY_MAX) / 1000.0
    await asyncio.sleep(delay)
    
    # Generate mock images
    images = []
    for i in range(request.n):
        seed = request.seed + i if request.seed else None
        
        if request.response_format == "b64_json":
            image_data = generate_mock_image(request.width, request.height, seed)
            images.append({
                "b64_json": image_data,
                "revised_prompt": f"[{MOCK_NAME}] {request.prompt}",
                "seed": seed or random.randint(0, 2**32)
            })
        else:
            # For URL format, return a placeholder URL
            image_id = str(uuid.uuid4())
            images.append({
                "url": f"http://{MOCK_NAME}:8000/images/{image_id}.png",
                "revised_prompt": f"[{MOCK_NAME}] {request.prompt}",
                "seed": seed or random.randint(0, 2**32)
            })
    
    metrics["requests_success"] += 1
    response_time = (time.time() - start_time) * 1000
    
    # Update average response time
    total = metrics["requests_success"]
    metrics["avg_response_time"] = (
        (metrics["avg_response_time"] * (total - 1) + response_time) / total
    )
    
    return {
        "created": int(time.time()),
        "data": images,
        "model": request.model or "mock-sd-v1"
    }


@app.post("/generate")
async def generate_alt(request: GenerateRequest):
    """Alternative generation endpoint"""
    return await generate_images(request)


@app.post("/sdapi/v1/txt2img")
async def txt2img(request: Request):
    """Automatic1111-style endpoint"""
    body = await request.json()
    
    gen_request = GenerateRequest(
        prompt=body.get("prompt", ""),
        negative_prompt=body.get("negative_prompt"),
        n=body.get("batch_size", 1),
        width=body.get("width", 512),
        height=body.get("height", 512),
        seed=body.get("seed", -1) if body.get("seed", -1) != -1 else None,
        guidance_scale=body.get("cfg_scale", 7.5),
        num_inference_steps=body.get("steps", 50)
    )
    
    result = await generate_images(gen_request)
    
    # Convert to A1111 format
    return {
        "images": [img.get("b64_json", "") for img in result["data"]],
        "parameters": body,
        "info": str(result)
    }


@app.get("/")
async def root():
    """Root endpoint"""
    return {
        "name": MOCK_NAME,
        "type": "mock-image-backend",
        "endpoints": [
            "/health",
            "/metrics",
            "/v1/images/generations",
            "/generate",
            "/sdapi/v1/txt2img"
        ]
    }


if __name__ == "__main__":
    import uvicorn
    uvicorn.run(app, host="0.0.0.0", port=8000)

