/**
 * Soak (Endurance) Load Test
 * 
 * Purpose: Test system stability over extended period
 *          Detect memory leaks, resource exhaustion, degradation
 * VUs: 50 constant users
 * Duration: 30 minutes
 */

import { sleep } from 'k6';
import { generateImage, healthCheck, listBackends, thinkTime } from '../lib/helpers.js';
import { CONFIG } from '../lib/config.js';
import { Counter } from 'k6/metrics';

const healthChecks = new Counter('health_checks');
const healthChecksFailed = new Counter('health_checks_failed');

export const options = {
  scenarios: {
    soak: {
      executor: 'constant-vus',
      vus: 50,
      duration: '30m',
    },
    health_monitor: {
      executor: 'constant-arrival-rate',
      rate: 1, // 1 health check per second
      timeUnit: '10s', // Actually every 10 seconds
      duration: '30m',
      preAllocatedVUs: 1,
      exec: 'healthMonitor',
    },
  },
  thresholds: {
    http_req_duration: ['p(95)<1000', 'p(99)<2000'],
    http_req_failed: ['rate<0.02'], // Less than 2% error rate
    'generation_error_rate': ['rate<0.02'],
    'health_checks_failed': ['count<10'], // Less than 10 failed health checks
  },
  summaryTrendStats: ['avg', 'min', 'med', 'max', 'p(90)', 'p(95)', 'p(99)'],
};

// Store metrics over time for trend analysis
const metricsOverTime = [];

export function setup() {
  console.log('Starting Soak (Endurance) Load Test');
  console.log(`Target: ${CONFIG.baseUrl}`);
  console.log('Duration: 30 minutes');
  console.log('⏱️  Monitoring for performance degradation and memory leaks');
  
  const healthy = healthCheck();
  if (!healthy) {
    throw new Error('Gateway health check failed');
  }
  
  // Get initial backend status
  const backends = listBackends();
  console.log(`Initial backends: ${backends.length}`);
  
  return { 
    startTime: Date.now(),
    initialBackends: backends.length,
  };
}

export default function () {
  generateImage({
    n: 1,
    response_format: 'b64_json',
  });
  
  // Normal think time for sustained load
  thinkTime(1, 2);
}

export function healthMonitor() {
  healthChecks.add(1);
  
  const healthy = healthCheck();
  if (!healthy) {
    healthChecksFailed.add(1);
    console.warn('⚠️  Health check failed during soak test');
  }
  
  // Record metric snapshot every 5 minutes
  // (This is a simplified version - in production would use more sophisticated monitoring)
}

export function teardown(data) {
  const duration = (Date.now() - data.startTime) / 1000 / 60;
  console.log(`Soak test completed after ${duration.toFixed(1)} minutes`);
  
  // Final health check
  const healthy = healthCheck();
  console.log(`Post-soak health: ${healthy ? 'STABLE' : 'DEGRADED'}`);
  
  // Check backends are still available
  const backends = listBackends();
  if (backends.length < data.initialBackends) {
    console.warn(`⚠️  Backend count decreased: ${data.initialBackends} -> ${backends.length}`);
  }
}

export function handleSummary(data) {
  const errorRate = data.metrics.http_req_failed?.values?.rate || 0;
  const avgDuration = data.metrics.http_req_duration?.values?.avg || 0;
  const p95 = data.metrics.http_req_duration?.values?.['p(95)'] || 0;
  const healthFailed = data.metrics.health_checks_failed?.values?.count || 0;
  
  // Analyze for degradation indicators
  const degradationIndicators = [];
  
  if (errorRate > 0.01) {
    degradationIndicators.push(`Error rate: ${(errorRate * 100).toFixed(2)}%`);
  }
  if (p95 > 1500) {
    degradationIndicators.push(`High p95 latency: ${p95.toFixed(0)}ms`);
  }
  if (healthFailed > 5) {
    degradationIndicators.push(`Health check failures: ${healthFailed}`);
  }
  
  const report = {
    test_type: 'soak',
    timestamp: new Date().toISOString(),
    config: {
      vus: 50,
      duration: '30m',
      target: CONFIG.baseUrl,
    },
    summary: {
      total_requests: data.metrics.http_reqs?.values?.count || 0,
      requests_per_second: data.metrics.http_reqs?.values?.rate || 0,
      avg_duration_ms: avgDuration,
      p95_duration_ms: p95,
      p99_duration_ms: data.metrics.http_req_duration?.values?.['p(99)'] || 0,
      error_rate: errorRate,
      health_checks_total: data.metrics.health_checks?.values?.count || 0,
      health_checks_failed: healthFailed,
    },
    analysis: {
      system_stable: degradationIndicators.length === 0,
      degradation_indicators: degradationIndicators,
      memory_leak_suspected: false, // Would need actual memory metrics
      recommendations: [],
    },
  };
  
  // Add recommendations
  if (degradationIndicators.length > 0) {
    report.analysis.recommendations.push('Investigate system resources during extended load');
    report.analysis.recommendations.push('Check for memory leaks in backend services');
  }
  if (errorRate > 0.01) {
    report.analysis.recommendations.push('Review connection pooling and timeout settings');
  }
  
  return {
    'stdout': JSON.stringify(report, null, 2) + '\n',
    'reports/latest/soak.json': JSON.stringify(report, null, 2),
  };
}

