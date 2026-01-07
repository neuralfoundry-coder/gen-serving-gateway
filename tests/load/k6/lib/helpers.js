/**
 * k6 Helper Functions
 */

import http from 'k6/http';
import { check, sleep } from 'k6';
import { Counter, Rate, Trend } from 'k6/metrics';
import { CONFIG, getRandomPrompt, getRandomSize, getRandomBackend } from './config.js';

// Custom metrics
export const successfulGenerations = new Counter('successful_generations');
export const failedGenerations = new Counter('failed_generations');
export const errorRate = new Rate('generation_error_rate');
export const responseTime = new Trend('generation_response_time');

/**
 * Generate image via the gateway
 */
export function generateImage(options = {}) {
  const url = `${CONFIG.baseUrl}/v1/images/generations`;
  
  const payload = JSON.stringify({
    prompt: options.prompt || getRandomPrompt(),
    n: options.n || 1,
    size: options.size || getRandomSize(),
    response_format: options.response_format || 'b64_json',
    backend: options.backend || getRandomBackend(),
  });
  
  const params = {
    headers: {
      'Content-Type': 'application/json',
      'Authorization': `Bearer ${CONFIG.apiKey}`,
    },
    timeout: '120s',
    tags: {
      name: 'generate_image',
      backend: options.backend || 'auto',
    },
  };
  
  const startTime = Date.now();
  const response = http.post(url, payload, params);
  const duration = Date.now() - startTime;
  
  // Record custom metrics
  responseTime.add(duration);
  
  const success = check(response, {
    'status is 200': (r) => r.status === 200,
    'response has data': (r) => {
      try {
        const body = JSON.parse(r.body);
        return body.data && body.data.length > 0;
      } catch {
        return false;
      }
    },
    'response time < 30s': (r) => r.timings.duration < 30000,
  });
  
  if (success) {
    successfulGenerations.add(1);
    errorRate.add(0);
  } else {
    failedGenerations.add(1);
    errorRate.add(1);
    
    // Log error for debugging
    if (__ENV.DEBUG) {
      console.log(`Error: ${response.status} - ${response.body}`);
    }
  }
  
  return { response, success, duration };
}

/**
 * Health check
 */
export function healthCheck() {
  const response = http.get(`${CONFIG.baseUrl}/health`);
  
  return check(response, {
    'health check status is 200': (r) => r.status === 200,
    'gateway is healthy': (r) => {
      try {
        const body = JSON.parse(r.body);
        return body.status === 'healthy' || body.status === 'degraded';
      } catch {
        return false;
      }
    },
  });
}

/**
 * List backends
 */
export function listBackends() {
  const response = http.get(`${CONFIG.baseUrl}/v1/backends`, {
    headers: {
      'Authorization': `Bearer ${CONFIG.apiKey}`,
    },
  });
  
  check(response, {
    'backends list status is 200': (r) => r.status === 200,
  });
  
  try {
    return JSON.parse(response.body).backends || [];
  } catch {
    return [];
  }
}

/**
 * Random think time between requests
 */
export function thinkTime(min = 0.5, max = 2) {
  sleep(Math.random() * (max - min) + min);
}

/**
 * Generate summary report data
 */
export function generateReport(data) {
  return {
    timestamp: new Date().toISOString(),
    scenario: __ENV.SCENARIO || 'unknown',
    metrics: {
      total_requests: data.metrics.http_reqs?.values?.count || 0,
      success_rate: 1 - (data.metrics.http_req_failed?.values?.rate || 0),
      avg_response_time: data.metrics.http_req_duration?.values?.avg || 0,
      p95_response_time: data.metrics.http_req_duration?.values?.['p(95)'] || 0,
      p99_response_time: data.metrics.http_req_duration?.values?.['p(99)'] || 0,
      max_vus: data.metrics.vus?.values?.max || 0,
      iterations: data.metrics.iterations?.values?.count || 0,
    },
    thresholds: {
      passed: Object.entries(data.thresholds || {})
        .filter(([_, v]) => v.ok)
        .map(([k]) => k),
      failed: Object.entries(data.thresholds || {})
        .filter(([_, v]) => !v.ok)
        .map(([k]) => k),
    },
  };
}

