/**
 * Stress Load Test
 * 
 * Purpose: Find system limits and breaking points
 * Pattern: Gradual increase from 10 to 300 VUs
 */

import { sleep } from 'k6';
import { generateImage, healthCheck, thinkTime } from '../lib/helpers.js';
import { CONFIG } from '../lib/config.js';

export const options = {
  scenarios: {
    stress: {
      executor: 'ramping-vus',
      startVUs: 0,
      stages: [
        { duration: '1m', target: 10 },    // Warm up
        { duration: '2m', target: 10 },    // Stay at 10
        { duration: '1m', target: 50 },    // Ramp to 50
        { duration: '2m', target: 50 },    // Stay at 50
        { duration: '1m', target: 100 },   // Ramp to 100
        { duration: '2m', target: 100 },   // Stay at 100
        { duration: '1m', target: 200 },   // Ramp to 200
        { duration: '2m', target: 200 },   // Stay at 200
        { duration: '1m', target: 300 },   // Ramp to 300
        { duration: '2m', target: 300 },   // Stay at 300 (breaking point?)
        { duration: '2m', target: 0 },     // Cool down
      ],
      gracefulRampDown: '30s',
    },
  },
  thresholds: {
    http_req_duration: ['p(95)<5000'],      // 5s threshold
    http_req_failed: ['rate<0.2'],          // Allow up to 20% error
    'generation_error_rate': ['rate<0.2'],
  },
  summaryTrendStats: ['avg', 'min', 'med', 'max', 'p(90)', 'p(95)', 'p(99)'],
};

export function setup() {
  console.log('Starting Stress Load Test');
  console.log(`Target: ${CONFIG.baseUrl}`);
  console.log('⚠️  This test will push the system to its limits');
  
  const healthy = healthCheck();
  if (!healthy) {
    throw new Error('Gateway health check failed');
  }
  
  return { 
    startTime: Date.now(),
    checkpoints: [],
  };
}

export default function () {
  generateImage({
    n: 1,
    response_format: 'b64_json',
  });
  
  // Minimal think time for stress test
  thinkTime(0.1, 0.5);
}

export function teardown(data) {
  const duration = (Date.now() - data.startTime) / 1000;
  console.log(`Stress test completed in ${duration.toFixed(2)}s`);
  
  // Health check after stress
  const healthy = healthCheck();
  console.log(`Post-stress health: ${healthy ? 'RECOVERED' : 'DEGRADED'}`);
}

export function handleSummary(data) {
  const errorRate = data.metrics.http_req_failed?.values?.rate || 0;
  const p95 = data.metrics.http_req_duration?.values?.['p(95)'] || 0;
  const maxVUs = data.metrics.vus?.values?.max || 0;
  
  // Estimate breaking point
  let breakingPointEstimate = 'Not reached';
  if (errorRate > 0.1) {
    breakingPointEstimate = `Approximately ${maxVUs} VUs (error rate: ${(errorRate * 100).toFixed(1)}%)`;
  }
  
  const report = {
    test_type: 'stress',
    timestamp: new Date().toISOString(),
    config: {
      max_vus: 300,
      stages: '10 -> 50 -> 100 -> 200 -> 300 VUs',
      target: CONFIG.baseUrl,
    },
    summary: {
      total_requests: data.metrics.http_reqs?.values?.count || 0,
      requests_per_second: data.metrics.http_reqs?.values?.rate || 0,
      avg_duration_ms: data.metrics.http_req_duration?.values?.avg || 0,
      p95_duration_ms: p95,
      p99_duration_ms: data.metrics.http_req_duration?.values?.['p(99)'] || 0,
      max_duration_ms: data.metrics.http_req_duration?.values?.max || 0,
      error_rate: errorRate,
      max_vus: maxVUs,
    },
    analysis: {
      breaking_point_estimate: breakingPointEstimate,
      system_stable: errorRate < 0.1 && p95 < 5000,
      recommendations: [],
    },
  };
  
  // Add recommendations based on results
  if (errorRate > 0.05) {
    report.analysis.recommendations.push('Consider scaling resources or optimizing backend');
  }
  if (p95 > 2000) {
    report.analysis.recommendations.push('Response times are high - investigate bottlenecks');
  }
  
  return {
    'stdout': JSON.stringify(report, null, 2) + '\n',
    'reports/latest/stress.json': JSON.stringify(report, null, 2),
  };
}

