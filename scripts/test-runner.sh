#!/bin/bash
# Unified Test Runner Script
# Runs unit, functional, integration, and load tests

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
REPORTS_DIR="$PROJECT_DIR/reports"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Results tracking
RESULTS_FILE="$REPORTS_DIR/latest/test_results.json"
declare -A TEST_RESULTS

print_header() {
    echo ""
    echo -e "${BLUE}==================================================${NC}"
    echo -e "${BLUE}  $1${NC}"
    echo -e "${BLUE}==================================================${NC}"
    echo ""
}

print_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

print_error() {
    echo -e "${RED}❌ $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

# Create reports directory
setup_reports() {
    mkdir -p "$REPORTS_DIR/latest"
    mkdir -p "$REPORTS_DIR/history"
    
    # Archive previous results
    if [ -f "$RESULTS_FILE" ]; then
        local timestamp=$(date +%Y%m%d_%H%M%S)
        mv "$REPORTS_DIR/latest" "$REPORTS_DIR/history/$timestamp" 2>/dev/null || true
        mkdir -p "$REPORTS_DIR/latest"
    fi
}

# Run Rust unit tests
run_unit_tests() {
    print_header "Running Unit Tests"
    
    cd "$PROJECT_DIR"
    
    if cargo test --lib 2>&1 | tee "$REPORTS_DIR/latest/unit_tests.log"; then
        TEST_RESULTS["unit"]="passed"
        print_success "Unit tests passed"
        return 0
    else
        TEST_RESULTS["unit"]="failed"
        print_error "Unit tests failed"
        return 1
    fi
}

# Run functional tests
run_functional_tests() {
    print_header "Running Functional Tests"
    
    cd "$PROJECT_DIR"
    
    # Run tests from tests/functional directory
    if cargo test --test '*' functional 2>&1 | tee "$REPORTS_DIR/latest/functional_tests.log"; then
        TEST_RESULTS["functional"]="passed"
        print_success "Functional tests passed"
        return 0
    else
        TEST_RESULTS["functional"]="failed"
        print_error "Functional tests failed"
        return 1
    fi
}

# Run integration tests
run_integration_tests() {
    print_header "Running Integration Tests"
    
    cd "$PROJECT_DIR"
    
    if cargo test --test '*' integration 2>&1 | tee "$REPORTS_DIR/latest/integration_tests.log"; then
        TEST_RESULTS["integration"]="passed"
        print_success "Integration tests passed"
        return 0
    else
        TEST_RESULTS["integration"]="failed"
        print_error "Integration tests failed"
        return 1
    fi
}

# Run all Rust tests
run_rust_tests() {
    print_header "Running All Rust Tests"
    
    cd "$PROJECT_DIR"
    
    if cargo test 2>&1 | tee "$REPORTS_DIR/latest/all_rust_tests.log"; then
        TEST_RESULTS["rust"]="passed"
        print_success "All Rust tests passed"
        return 0
    else
        TEST_RESULTS["rust"]="failed"
        print_error "Some Rust tests failed"
        return 1
    fi
}

# Run load tests
run_load_tests() {
    local scenario="${1:-baseline}"
    
    print_header "Running Load Tests: $scenario"
    
    # Check if k6 is installed
    if ! command -v k6 &> /dev/null; then
        print_warning "k6 not installed. Skipping load tests."
        print_warning "Install with: brew install k6 (macOS) or see https://k6.io/docs/getting-started/installation/"
        TEST_RESULTS["load"]="skipped"
        return 0
    fi
    
    # Check if gateway is running
    if ! curl -s http://localhost:8080/health > /dev/null 2>&1; then
        print_warning "Gateway not running. Start with: docker-compose up -d"
        print_warning "Skipping load tests."
        TEST_RESULTS["load"]="skipped"
        return 0
    fi
    
    cd "$PROJECT_DIR/tests/load/k6"
    
    if ./run_all.sh --scenario "$scenario"; then
        TEST_RESULTS["load_$scenario"]="passed"
        print_success "Load test ($scenario) completed"
        return 0
    else
        TEST_RESULTS["load_$scenario"]="failed"
        print_error "Load test ($scenario) failed"
        return 1
    fi
}

# Run all load test scenarios
run_all_load_tests() {
    print_header "Running All Load Test Scenarios"
    
    local scenarios=("baseline" "spike" "stress")
    local failed=0
    
    for scenario in "${scenarios[@]}"; do
        run_load_tests "$scenario" || failed=$((failed + 1))
        sleep 5  # Brief pause between scenarios
    done
    
    return $failed
}

# Generate summary
generate_summary() {
    print_header "Test Summary"
    
    local total=0
    local passed=0
    local failed=0
    local skipped=0
    
    for test in "${!TEST_RESULTS[@]}"; do
        total=$((total + 1))
        case "${TEST_RESULTS[$test]}" in
            passed) passed=$((passed + 1)) ;;
            failed) failed=$((failed + 1)) ;;
            skipped) skipped=$((skipped + 1)) ;;
        esac
    done
    
    echo "Total:   $total"
    echo -e "${GREEN}Passed:  $passed${NC}"
    echo -e "${RED}Failed:  $failed${NC}"
    echo -e "${YELLOW}Skipped: $skipped${NC}"
    echo ""
    
    # Save results to JSON
    local timestamp=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
    cat > "$RESULTS_FILE" << EOF
{
  "timestamp": "$timestamp",
  "summary": {
    "total": $total,
    "passed": $passed,
    "failed": $failed,
    "skipped": $skipped
  },
  "results": {
EOF
    
    local first=true
    for test in "${!TEST_RESULTS[@]}"; do
        if [ "$first" = true ]; then
            first=false
        else
            echo "," >> "$RESULTS_FILE"
        fi
        echo -n "    \"$test\": \"${TEST_RESULTS[$test]}\"" >> "$RESULTS_FILE"
    done
    
    cat >> "$RESULTS_FILE" << EOF

  }
}
EOF
    
    echo "Results saved to: $RESULTS_FILE"
    
    return $failed
}

# Print usage
usage() {
    echo "Usage: $0 [command] [options]"
    echo ""
    echo "Commands:"
    echo "  all           Run all tests (unit, functional, integration)"
    echo "  unit          Run unit tests only"
    echo "  functional    Run functional tests only"
    echo "  integration   Run integration tests only"
    echo "  rust          Run all Rust tests"
    echo "  load          Run load tests (requires k6 and running gateway)"
    echo ""
    echo "Options:"
    echo "  --scenario NAME   Load test scenario (baseline, spike, stress, soak, breakpoint)"
    echo "  --all-load        Run all load test scenarios"
    echo "  --help            Show this help"
    echo ""
    echo "Examples:"
    echo "  $0 all                        # Run all Rust tests"
    echo "  $0 load --scenario baseline   # Run baseline load test"
    echo "  $0 load --all-load            # Run all load tests"
}

# Main
main() {
    local command="${1:-all}"
    shift || true
    
    setup_reports
    
    case "$command" in
        all)
            run_rust_tests
            ;;
        unit)
            run_unit_tests
            ;;
        functional)
            run_functional_tests
            ;;
        integration)
            run_integration_tests
            ;;
        rust)
            run_rust_tests
            ;;
        load)
            while [[ $# -gt 0 ]]; do
                case $1 in
                    --scenario)
                        run_load_tests "$2"
                        shift 2
                        ;;
                    --all-load)
                        run_all_load_tests
                        shift
                        ;;
                    *)
                        run_load_tests "$1"
                        shift
                        ;;
                esac
            done
            ;;
        --help|-h|help)
            usage
            exit 0
            ;;
        *)
            echo "Unknown command: $command"
            usage
            exit 1
            ;;
    esac
    
    generate_summary
}

main "$@"

