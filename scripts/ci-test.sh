#!/bin/bash
# CI/CD Test Script
# Runs all tests and generates reports in CI environment

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

echo "=================================================="
echo "  CI Test Pipeline"
echo "=================================================="
echo "Project: $PROJECT_DIR"
echo "Time: $(date)"
echo ""

# Step 1: Run Rust tests
echo "📦 Step 1: Running Rust tests..."
cd "$PROJECT_DIR"
cargo test --all 2>&1 | tee test_output.log

if [ ${PIPESTATUS[0]} -ne 0 ]; then
    echo "❌ Rust tests failed"
    exit 1
fi
echo "✅ Rust tests passed"

# Step 2: Check if load tests should run
if [ "$RUN_LOAD_TESTS" = "true" ]; then
    echo ""
    echo "🔄 Step 2: Running load tests..."
    
    # Check if k6 is available
    if command -v k6 &> /dev/null; then
        # Check if services are running
        if curl -s http://localhost:8080/health > /dev/null 2>&1; then
            ./scripts/test-runner.sh load --scenario baseline
        else
            echo "⚠️  Gateway not running, skipping load tests"
        fi
    else
        echo "⚠️  k6 not installed, skipping load tests"
    fi
else
    echo ""
    echo "⏭️  Step 2: Skipping load tests (RUN_LOAD_TESTS not set)"
fi

# Step 3: Generate reports if load tests ran
if [ -f "$PROJECT_DIR/reports/latest/baseline.json" ]; then
    echo ""
    echo "📊 Step 3: Generating reports..."
    
    python3 "$SCRIPT_DIR/analyze-results.py" --generate-report \
        -o "$PROJECT_DIR/reports/latest/improvements.md"
    
    python3 "$SCRIPT_DIR/generate-report.py" \
        -o "$PROJECT_DIR/reports/latest/dashboard.html"
    
    python3 "$SCRIPT_DIR/feedback-loop.py"
    
    echo "✅ Reports generated"
else
    echo ""
    echo "⏭️  Step 3: Skipping report generation (no load test results)"
fi

# Step 4: Summary
echo ""
echo "=================================================="
echo "  Pipeline Summary"
echo "=================================================="
echo "✅ All tests completed successfully"

# Output any issues found
if [ -f "$PROJECT_DIR/reports/latest/analysis.json" ]; then
    echo ""
    echo "📋 Issues Summary:"
    python3 -c "
import json
with open('$PROJECT_DIR/reports/latest/analysis.json') as f:
    data = json.load(f)
    counts = data.get('issue_count', {})
    print(f'   Critical: {counts.get(\"critical\", 0)}')
    print(f'   High: {counts.get(\"high\", 0)}')
    print(f'   Medium: {counts.get(\"medium\", 0)}')
    print(f'   Low: {counts.get(\"low\", 0)}')
"
fi

echo ""
echo "Done!"

