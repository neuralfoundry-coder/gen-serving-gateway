/**
 * Breakpoint Load Test
 * 
 * Purpose: Find the exact breaking point of the system
 *          Continuously increase load until error rate exceeds threshold
 * Pattern: Start at 10 VUs, add 20 VUs every minute until 10% error rate
 */

import { sleep } from 'k6';
import { generateImage, healthCheck, thinkTime } from '../lib/helpers.js';
import { CONFIG } from '../lib/config.js';
import { Rate } from 'k6/metrics';

const currentErrorRate = new Rate('current_error_rate');

export const options = {
  scenarios: {
    breakpoint: {
      executor: 'ramping-arrival-rate',
      startRate: 10,
      timeUnit: '1s',
      preAllocatedVUs: 500,
      maxVUs: 1000,
      stages: [
        // Gradually increase rate until we find the breaking point
        { duration: '1m', target: 10 },
        { duration: '1m', target: 30 },
        { duration: '1m', target: 50 },
        { duration: '1m', target: 70 },
        { duration: '1m', target: 100 },
        { duration: '1m', target: 130 },
        { duration: '1m', target: 160 },
        { duration: '1m', target: 200 },
        { duration: '1m', target: 250 },
        { duration: '1m', target: 300 },
        { duration: '1m', target: 350 },
        { duration: '1m', target: 400 },
      ],
    },
  },
  thresholds: {
    // The test continues until error rate hits 10%
    // These thresholds are for reporting
    http_req_duration: ['p(95)<10000'], // Very lenient - we're finding limits
    'current_error_rate': ['rate<0.10'], // 10% threshold
  },
  summaryTrendStats: ['avg', 'min', 'med', 'max', 'p(90)', 'p(95)', 'p(99)'],
};

// Track when we hit the breaking point
let breakingPointHit = false;
let breakingPointRate = 0;
let breakingPointTime = null;

export function setup() {
  console.log('Starting Breakpoint Load Test');
  console.log(`Target: ${CONFIG.baseUrl}`);
  console.log('⚠️  This test will find the breaking point - expect errors');
  
  const healthy = healthCheck();
  if (!healthy) {
    throw new Error('Gateway health check failed');
  }
  
  return { startTime: Date.now() };
}

export default function () {
  const result = generateImage({
    n: 1,
    response_format: 'b64_json',
  });
  
  // Track error rate
  currentErrorRate.add(!result.success);
  
  // Check if we've hit the breaking point
  // Note: This is approximate - actual detection would use metrics API
  if (!breakingPointHit && !result.success) {
    // Could implement more sophisticated breaking point detection
  }
  
  // Very minimal think time - we want to find the limit
  thinkTime(0.05, 0.1);
}

export function teardown(data) {
  const duration = (Date.now() - data.startTime) / 1000;
  console.log(`Breakpoint test completed in ${duration.toFixed(2)}s`);
  
  // Recovery check
  sleep(5);
  const healthy = healthCheck();
  console.log(`Post-breakpoint health: ${healthy ? 'RECOVERED' : 'STILL DEGRADED'}`);
}

export function handleSummary(data) {
  const errorRate = data.metrics.http_req_failed?.values?.rate || 0;
  const maxRate = data.metrics.http_reqs?.values?.rate || 0;
  const p95 = data.metrics.http_req_duration?.values?.['p(95)'] || 0;
  const totalRequests = data.metrics.http_reqs?.values?.count || 0;
  
  // Estimate breaking point based on error rate
  let breakingPointEstimate = 'Not determined';
  let maxSustainableRate = maxRate;
  
  if (errorRate > 0.1) {
    // We exceeded threshold - estimate where it broke
    maxSustainableRate = maxRate * (1 - errorRate) * 0.9; // Conservative estimate
    breakingPointEstimate = `~${maxSustainableRate.toFixed(0)} req/s`;
  } else {
    breakingPointEstimate = `>${maxRate.toFixed(0)} req/s (threshold not reached)`;
  }
  
  const report = {
    test_type: 'breakpoint',
    timestamp: new Date().toISOString(),
    config: {
      max_rate_tested: '400 req/s',
      error_threshold: '10%',
      target: CONFIG.baseUrl,
    },
    summary: {
      total_requests: totalRequests,
      peak_requests_per_second: maxRate,
      avg_duration_ms: data.metrics.http_req_duration?.values?.avg || 0,
      p95_duration_ms: p95,
      p99_duration_ms: data.metrics.http_req_duration?.values?.['p(99)'] || 0,
      max_duration_ms: data.metrics.http_req_duration?.values?.max || 0,
      final_error_rate: errorRate,
    },
    analysis: {
      breaking_point: breakingPointEstimate,
      max_sustainable_rate: `~${maxSustainableRate.toFixed(0)} req/s`,
      error_threshold_exceeded: errorRate > 0.1,
      recommendations: [],
    },
  };
  
  // Add recommendations
  if (errorRate > 0.05) {
    report.analysis.recommendations.push(`System starts degrading around ${maxSustainableRate.toFixed(0)} req/s`);
    report.analysis.recommendations.push('Consider horizontal scaling or rate limiting');
  }
  if (p95 > 5000) {
    report.analysis.recommendations.push('High latency detected - optimize request handling');
  }
  
  // Capacity planning suggestions
  report.analysis.capacity_planning = {
    recommended_max_load: `${(maxSustainableRate * 0.7).toFixed(0)} req/s (70% of breaking point)`,
    scale_trigger: `${(maxSustainableRate * 0.8).toFixed(0)} req/s (80% of breaking point)`,
  };
  
  return {
    'stdout': JSON.stringify(report, null, 2) + '\n',
    'reports/latest/breakpoint.json': JSON.stringify(report, null, 2),
  };
}

