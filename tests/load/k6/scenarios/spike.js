/**
 * Spike Load Test
 * 
 * Purpose: Test system behavior under sudden traffic spikes
 * Pattern: 10 VUs -> 100 VUs (spike) -> 100 VUs (hold) -> 10 VUs (recovery)
 */

import { sleep } from 'k6';
import { generateImage, healthCheck, thinkTime } from '../lib/helpers.js';
import { CONFIG } from '../lib/config.js';

export const options = {
  scenarios: {
    spike: {
      executor: 'ramping-vus',
      startVUs: 10,
      stages: [
        { duration: '30s', target: 10 },   // Warm up
        { duration: '30s', target: 100 },  // Spike up
        { duration: '1m', target: 100 },   // Hold at peak
        { duration: '30s', target: 10 },   // Spike down
        { duration: '30s', target: 10 },   // Recovery
      ],
      gracefulRampDown: '10s',
    },
  },
  thresholds: {
    http_req_duration: ['p(95)<2000', 'p(99)<5000'], // More lenient during spike
    http_req_failed: ['rate<0.1'], // Allow up to 10% error during spike
    'generation_error_rate': ['rate<0.1'],
  },
  summaryTrendStats: ['avg', 'min', 'med', 'max', 'p(90)', 'p(95)', 'p(99)'],
};

export function setup() {
  console.log('Starting Spike Load Test');
  console.log(`Target: ${CONFIG.baseUrl}`);
  
  const healthy = healthCheck();
  if (!healthy) {
    throw new Error('Gateway health check failed');
  }
  
  return { startTime: Date.now() };
}

export default function () {
  generateImage({
    n: 1,
    response_format: 'b64_json',
  });
  
  // Shorter think time during spike test
  thinkTime(0.5, 1.5);
}

export function teardown(data) {
  const duration = (Date.now() - data.startTime) / 1000;
  console.log(`Spike test completed in ${duration.toFixed(2)}s`);
  
  // Final health check
  const healthy = healthCheck();
  console.log(`Post-test health: ${healthy ? 'OK' : 'DEGRADED'}`);
}

export function handleSummary(data) {
  const report = {
    test_type: 'spike',
    timestamp: new Date().toISOString(),
    config: {
      stages: [
        { duration: '30s', target: 10 },
        { duration: '30s', target: 100 },
        { duration: '1m', target: 100 },
        { duration: '30s', target: 10 },
        { duration: '30s', target: 10 },
      ],
      target: CONFIG.baseUrl,
    },
    summary: {
      total_requests: data.metrics.http_reqs?.values?.count || 0,
      requests_per_second: data.metrics.http_reqs?.values?.rate || 0,
      avg_duration_ms: data.metrics.http_req_duration?.values?.avg || 0,
      p95_duration_ms: data.metrics.http_req_duration?.values?.['p(95)'] || 0,
      p99_duration_ms: data.metrics.http_req_duration?.values?.['p(99)'] || 0,
      max_duration_ms: data.metrics.http_req_duration?.values?.max || 0,
      error_rate: data.metrics.http_req_failed?.values?.rate || 0,
      max_vus: data.metrics.vus?.values?.max || 0,
    },
    analysis: {
      spike_handled: (data.metrics.http_req_failed?.values?.rate || 0) < 0.1,
      recovery_time_estimate: 'Check p99 during recovery phase',
    },
  };
  
  return {
    'stdout': JSON.stringify(report, null, 2) + '\n',
    'reports/latest/spike.json': JSON.stringify(report, null, 2),
  };
}

