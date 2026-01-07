/**
 * Baseline Load Test
 * 
 * Purpose: Establish baseline performance metrics under normal load
 * VUs: 10 constant users
 * Duration: 5 minutes
 */

import { sleep } from 'k6';
import { generateImage, healthCheck, thinkTime } from '../lib/helpers.js';
import { CONFIG } from '../lib/config.js';

export const options = {
  scenarios: {
    baseline: {
      executor: 'constant-vus',
      vus: 10,
      duration: '5m',
    },
  },
  thresholds: {
    http_req_duration: ['p(95)<500', 'p(99)<1000'],
    http_req_failed: ['rate<0.01'], // Less than 1% error rate
    'generation_error_rate': ['rate<0.01'],
  },
  // Output to JSON for analysis
  summaryTrendStats: ['avg', 'min', 'med', 'max', 'p(90)', 'p(95)', 'p(99)'],
};

export function setup() {
  console.log('Starting Baseline Load Test');
  console.log(`Target: ${CONFIG.baseUrl}`);
  
  // Verify gateway is healthy
  const healthy = healthCheck();
  if (!healthy) {
    throw new Error('Gateway health check failed');
  }
  
  return { startTime: Date.now() };
}

export default function () {
  // Generate an image
  const result = generateImage({
    n: 1,
    response_format: 'b64_json',
  });
  
  // Think time between requests
  thinkTime(1, 3);
}

export function teardown(data) {
  const duration = (Date.now() - data.startTime) / 1000;
  console.log(`Baseline test completed in ${duration.toFixed(2)}s`);
}

export function handleSummary(data) {
  const report = {
    test_type: 'baseline',
    timestamp: new Date().toISOString(),
    config: {
      vus: 10,
      duration: '5m',
      target: CONFIG.baseUrl,
    },
    summary: {
      total_requests: data.metrics.http_reqs?.values?.count || 0,
      requests_per_second: data.metrics.http_reqs?.values?.rate || 0,
      avg_duration_ms: data.metrics.http_req_duration?.values?.avg || 0,
      p95_duration_ms: data.metrics.http_req_duration?.values?.['p(95)'] || 0,
      p99_duration_ms: data.metrics.http_req_duration?.values?.['p(99)'] || 0,
      error_rate: data.metrics.http_req_failed?.values?.rate || 0,
      successful_generations: data.metrics.successful_generations?.values?.count || 0,
      failed_generations: data.metrics.failed_generations?.values?.count || 0,
    },
    thresholds: data.root_group?.checks || {},
  };
  
  return {
    'stdout': JSON.stringify(report, null, 2) + '\n',
    'reports/latest/baseline.json': JSON.stringify(report, null, 2),
  };
}

