/**
 * k6 Test Configuration
 */

export const CONFIG = {
  // Target gateway URL
  baseUrl: __ENV.BASE_URL || 'http://localhost:8080',
  
  // API Key for authentication
  apiKey: __ENV.API_KEY || 'test-api-key-1',
  
  // Default thresholds
  thresholds: {
    http_req_duration: ['p(95)<500', 'p(99)<1000'],
    http_req_failed: ['rate<0.05'],
    http_reqs: ['rate>10'],
  },
  
  // Test prompts for variety
  prompts: [
    'A beautiful sunset over mountains with golden light',
    'A futuristic city with flying cars and neon lights',
    'A serene Japanese garden with cherry blossoms',
    'An underwater scene with colorful coral reef',
    'A cozy cabin in snowy forest during winter',
    'An abstract digital art with geometric patterns',
    'A steampunk robot in a Victorian library',
    'A magical forest with glowing mushrooms',
    'A space station orbiting Earth at night',
    'A vintage car on Route 66 at sunset',
  ],
  
  // Image sizes to test
  sizes: ['512x512', '768x768', '1024x1024'],
  
  // Backends to target
  backends: ['http-mock-1', 'http-mock-2', null], // null = let gateway decide
};

export function getRandomPrompt() {
  return CONFIG.prompts[Math.floor(Math.random() * CONFIG.prompts.length)];
}

export function getRandomSize() {
  return CONFIG.sizes[Math.floor(Math.random() * CONFIG.sizes.length)];
}

export function getRandomBackend() {
  return CONFIG.backends[Math.floor(Math.random() * CONFIG.backends.length)];
}

