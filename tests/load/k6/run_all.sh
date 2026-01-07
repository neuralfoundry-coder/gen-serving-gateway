#!/bin/bash
# Run all k6 load test scenarios

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$(dirname "$(dirname "$SCRIPT_DIR")")")"
REPORTS_DIR="$PROJECT_DIR/reports/latest"

# Ensure reports directory exists
mkdir -p "$REPORTS_DIR"

# Default configuration
BASE_URL="${BASE_URL:-http://localhost:8080}"
API_KEY="${API_KEY:-test-api-key-1}"

echo "=================================================="
echo "  k6 Load Test Suite"
echo "=================================================="
echo "Target: $BASE_URL"
echo "Reports: $REPORTS_DIR"
echo ""

# Function to run a scenario
run_scenario() {
    local scenario=$1
    local description=$2
    
    echo ""
    echo "=================================================="
    echo "Running: $description"
    echo "Scenario: $scenario"
    echo "=================================================="
    
    k6 run \
        -e BASE_URL="$BASE_URL" \
        -e API_KEY="$API_KEY" \
        -e SCENARIO="$scenario" \
        --out json="$REPORTS_DIR/${scenario}_raw.json" \
        "$SCRIPT_DIR/scenarios/${scenario}.js" \
        2>&1 | tee "$REPORTS_DIR/${scenario}_output.log"
    
    local exit_code=${PIPESTATUS[0]}
    
    if [ $exit_code -eq 0 ]; then
        echo "✅ $scenario completed successfully"
    else
        echo "❌ $scenario failed with exit code $exit_code"
    fi
    
    return $exit_code
}

# Parse arguments
SCENARIOS=""
RUN_ALL=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --all)
            RUN_ALL=true
            shift
            ;;
        --scenario)
            SCENARIOS="$SCENARIOS $2"
            shift 2
            ;;
        --url)
            BASE_URL="$2"
            shift 2
            ;;
        --help)
            echo "Usage: $0 [options]"
            echo ""
            echo "Options:"
            echo "  --all              Run all scenarios"
            echo "  --scenario NAME    Run specific scenario (baseline, spike, stress, soak, breakpoint)"
            echo "  --url URL          Target URL (default: http://localhost:8080)"
            echo "  --help             Show this help"
            exit 0
            ;;
        *)
            SCENARIOS="$SCENARIOS $1"
            shift
            ;;
    esac
done

# If no scenarios specified, show menu
if [ -z "$SCENARIOS" ] && [ "$RUN_ALL" = false ]; then
    echo "Available scenarios:"
    echo "  1. baseline    - Normal load baseline (5 min)"
    echo "  2. spike       - Spike load test (3 min)"
    echo "  3. stress      - Stress test to find limits (17 min)"
    echo "  4. soak        - Endurance test (30 min)"
    echo "  5. breakpoint  - Find breaking point (12 min)"
    echo ""
    echo "Usage: $0 --scenario baseline"
    echo "       $0 --all"
    exit 0
fi

# Run scenarios
FAILED=0

if [ "$RUN_ALL" = true ]; then
    SCENARIOS="baseline spike stress"
    # Note: soak and breakpoint are excluded from --all due to duration
    echo "⚠️  Running baseline, spike, and stress tests"
    echo "   For soak and breakpoint, run explicitly: --scenario soak"
fi

for scenario in $SCENARIOS; do
    case $scenario in
        baseline)
            run_scenario baseline "Baseline Load Test (5 min)" || FAILED=$((FAILED + 1))
            ;;
        spike)
            run_scenario spike "Spike Load Test (3 min)" || FAILED=$((FAILED + 1))
            ;;
        stress)
            run_scenario stress "Stress Load Test (17 min)" || FAILED=$((FAILED + 1))
            ;;
        soak)
            run_scenario soak "Soak/Endurance Test (30 min)" || FAILED=$((FAILED + 1))
            ;;
        breakpoint)
            run_scenario breakpoint "Breakpoint Test (12 min)" || FAILED=$((FAILED + 1))
            ;;
        *)
            echo "Unknown scenario: $scenario"
            FAILED=$((FAILED + 1))
            ;;
    esac
done

echo ""
echo "=================================================="
echo "  Test Suite Complete"
echo "=================================================="

if [ $FAILED -gt 0 ]; then
    echo "❌ $FAILED scenario(s) failed"
    exit 1
else
    echo "✅ All scenarios completed successfully"
    echo ""
    echo "Reports saved to: $REPORTS_DIR"
fi

