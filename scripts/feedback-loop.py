#!/usr/bin/env python3
"""
Feedback Loop Implementation
Automatically analyzes test results, generates improvements documentation,
and creates actionable tasks based on performance metrics.
"""

import json
import os
import subprocess
import sys
from datetime import datetime
from pathlib import Path
from typing import Dict, List, Any, Optional
import argparse


class FeedbackLoop:
    """Implements continuous improvement feedback loop"""
    
    def __init__(self, project_dir: str):
        self.project_dir = Path(project_dir)
        self.reports_dir = self.project_dir / "reports"
        self.latest_dir = self.reports_dir / "latest"
        self.history_dir = self.reports_dir / "history"
        
        # Performance thresholds
        self.thresholds = {
            "p95_duration_ms": 500,
            "p99_duration_ms": 1000,
            "error_rate": 0.02,  # 2%
            "min_throughput": 50,  # req/s
        }
        
        # Improvement categories
        self.categories = {
            "critical": [],
            "high": [],
            "medium": [],
            "low": [],
        }
    
    def load_latest_results(self) -> Dict[str, Dict]:
        """Load all latest test results"""
        results = {}
        scenarios = ["baseline", "spike", "stress", "soak", "breakpoint"]
        
        for scenario in scenarios:
            result_file = self.latest_dir / f"{scenario}.json"
            if result_file.exists():
                with open(result_file) as f:
                    results[scenario] = json.load(f)
        
        return results
    
    def analyze_results(self, results: Dict[str, Dict]) -> Dict:
        """Analyze results and identify issues"""
        issues = []
        improvements = []
        metrics_summary = {}
        
        for scenario, data in results.items():
            summary = data.get("summary", {})
            analysis = data.get("analysis", {})
            
            metrics_summary[scenario] = {
                "avg_duration_ms": summary.get("avg_duration_ms", 0),
                "p95_duration_ms": summary.get("p95_duration_ms", 0),
                "p99_duration_ms": summary.get("p99_duration_ms", 0),
                "error_rate": summary.get("error_rate", 0),
                "throughput": summary.get("requests_per_second", 0),
            }
            
            # Check against thresholds
            p95 = summary.get("p95_duration_ms", 0)
            p99 = summary.get("p99_duration_ms", 0)
            error_rate = summary.get("error_rate", 0)
            throughput = summary.get("requests_per_second", 0)
            
            # P95 latency check
            if p95 > self.thresholds["p95_duration_ms"]:
                severity = "critical" if p95 > self.thresholds["p95_duration_ms"] * 2 else "high"
                issues.append({
                    "severity": severity,
                    "scenario": scenario,
                    "type": "latency",
                    "metric": "p95_duration_ms",
                    "value": p95,
                    "threshold": self.thresholds["p95_duration_ms"],
                    "description": f"P95 응답 시간이 임계값 초과 ({p95:.0f}ms > {self.thresholds['p95_duration_ms']}ms)",
                })
            
            # P99 latency check
            if p99 > self.thresholds["p99_duration_ms"]:
                severity = "critical" if p99 > self.thresholds["p99_duration_ms"] * 2 else "high"
                issues.append({
                    "severity": severity,
                    "scenario": scenario,
                    "type": "latency",
                    "metric": "p99_duration_ms",
                    "value": p99,
                    "threshold": self.thresholds["p99_duration_ms"],
                    "description": f"P99 응답 시간이 임계값 초과 ({p99:.0f}ms > {self.thresholds['p99_duration_ms']}ms)",
                })
            
            # Error rate check
            if error_rate > self.thresholds["error_rate"]:
                severity = "critical" if error_rate > 0.1 else "high"
                issues.append({
                    "severity": severity,
                    "scenario": scenario,
                    "type": "reliability",
                    "metric": "error_rate",
                    "value": error_rate,
                    "threshold": self.thresholds["error_rate"],
                    "description": f"에러율이 임계값 초과 ({error_rate*100:.2f}% > {self.thresholds['error_rate']*100}%)",
                })
            
            # Throughput check
            if throughput < self.thresholds["min_throughput"] and throughput > 0:
                issues.append({
                    "severity": "medium",
                    "scenario": scenario,
                    "type": "performance",
                    "metric": "throughput",
                    "value": throughput,
                    "threshold": self.thresholds["min_throughput"],
                    "description": f"처리량이 기준 미달 ({throughput:.1f} < {self.thresholds['min_throughput']} req/s)",
                })
            
            # Add scenario-specific recommendations
            for rec in analysis.get("recommendations", []):
                improvements.append({
                    "scenario": scenario,
                    "recommendation": rec,
                })
        
        return {
            "timestamp": datetime.now().isoformat(),
            "metrics_summary": metrics_summary,
            "issues": issues,
            "improvements": improvements,
            "issue_count": {
                "critical": len([i for i in issues if i["severity"] == "critical"]),
                "high": len([i for i in issues if i["severity"] == "high"]),
                "medium": len([i for i in issues if i["severity"] == "medium"]),
                "low": len([i for i in issues if i["severity"] == "low"]),
            }
        }
    
    def generate_action_items(self, analysis: Dict) -> List[Dict]:
        """Generate actionable improvement items"""
        action_items = []
        
        issues = analysis.get("issues", [])
        
        # Group issues by type
        latency_issues = [i for i in issues if i["type"] == "latency"]
        reliability_issues = [i for i in issues if i["type"] == "reliability"]
        performance_issues = [i for i in issues if i["type"] == "performance"]
        
        # Generate action items for latency issues
        if latency_issues:
            action_items.append({
                "id": "ACTION-001",
                "title": "응답 시간 최적화",
                "priority": "high" if any(i["severity"] == "critical" for i in latency_issues) else "medium",
                "description": "높은 응답 시간이 감지되었습니다.",
                "tasks": [
                    "백엔드 서버 응답 시간 프로파일링",
                    "데이터베이스 쿼리 최적화 검토",
                    "캐싱 전략 구현 또는 개선",
                    "연결 풀링 설정 검토",
                ],
                "affected_scenarios": list(set(i["scenario"] for i in latency_issues)),
                "metrics": {
                    "current_p95": max(i["value"] for i in latency_issues),
                    "target_p95": self.thresholds["p95_duration_ms"],
                }
            })
        
        # Generate action items for reliability issues
        if reliability_issues:
            action_items.append({
                "id": "ACTION-002",
                "title": "신뢰성 개선",
                "priority": "critical" if any(i["severity"] == "critical" for i in reliability_issues) else "high",
                "description": "높은 에러율이 감지되었습니다.",
                "tasks": [
                    "에러 로그 분석 및 근본 원인 파악",
                    "백엔드 헬스체크 강화",
                    "재시도 로직 및 서킷 브레이커 검토",
                    "타임아웃 설정 최적화",
                ],
                "affected_scenarios": list(set(i["scenario"] for i in reliability_issues)),
                "metrics": {
                    "current_error_rate": max(i["value"] for i in reliability_issues),
                    "target_error_rate": self.thresholds["error_rate"],
                }
            })
        
        # Generate action items for performance issues
        if performance_issues:
            action_items.append({
                "id": "ACTION-003",
                "title": "처리량 개선",
                "priority": "medium",
                "description": "처리량이 기준에 미달합니다.",
                "tasks": [
                    "동시성 설정 검토 및 최적화",
                    "리소스 제한 확인 (CPU, 메모리)",
                    "로드 밸런싱 전략 검토",
                    "수평적 확장 고려",
                ],
                "affected_scenarios": list(set(i["scenario"] for i in performance_issues)),
                "metrics": {
                    "current_throughput": min(i["value"] for i in performance_issues),
                    "target_throughput": self.thresholds["min_throughput"],
                }
            })
        
        return action_items
    
    def generate_improvements_document(self, analysis: Dict, action_items: List[Dict]) -> str:
        """Generate markdown document with improvements"""
        
        doc = f"""# 테스트 결과 분석 및 개선 계획

생성 시간: {analysis['timestamp']}

## 1. 요약

### 발견된 이슈
- 🔴 Critical: {analysis['issue_count']['critical']}건
- 🟠 High: {analysis['issue_count']['high']}건
- 🟡 Medium: {analysis['issue_count']['medium']}건
- 🟢 Low: {analysis['issue_count']['low']}건

### 성능 메트릭 요약

| 시나리오 | 평균 응답 | P95 응답 | P99 응답 | 에러율 | 처리량 |
|---------|----------|---------|---------|-------|--------|
"""
        
        for scenario, metrics in analysis['metrics_summary'].items():
            doc += f"| {scenario} | {metrics['avg_duration_ms']:.0f}ms | {metrics['p95_duration_ms']:.0f}ms | {metrics['p99_duration_ms']:.0f}ms | {metrics['error_rate']*100:.2f}% | {metrics['throughput']:.1f}/s |\n"
        
        doc += """
## 2. 발견된 이슈 상세

"""
        
        for issue in analysis['issues']:
            severity_icon = {
                "critical": "🔴",
                "high": "🟠",
                "medium": "🟡",
                "low": "🟢"
            }.get(issue['severity'], "⚪")
            
            doc += f"""### {severity_icon} [{issue['severity'].upper()}] {issue['description']}

- **시나리오**: {issue['scenario']}
- **메트릭**: {issue['metric']}
- **현재 값**: {issue['value']:.2f}
- **임계값**: {issue['threshold']}

"""
        
        doc += """## 3. 조치 계획

"""
        
        for item in action_items:
            priority_icon = {
                "critical": "🔴",
                "high": "🟠",
                "medium": "🟡",
                "low": "🟢"
            }.get(item['priority'], "⚪")
            
            doc += f"""### {priority_icon} {item['id']}: {item['title']}

**우선순위**: {item['priority'].upper()}
**설명**: {item['description']}
**영향 시나리오**: {', '.join(item['affected_scenarios'])}

**세부 작업**:
"""
            for task in item['tasks']:
                doc += f"- [ ] {task}\n"
            
            doc += f"""
**목표 메트릭**:
"""
            for metric, value in item['metrics'].items():
                if isinstance(value, float):
                    doc += f"- {metric}: {value:.2f}\n"
                else:
                    doc += f"- {metric}: {value}\n"
            
            doc += "\n"
        
        doc += """## 4. 다음 단계

1. 위 조치 계획을 검토하고 우선순위를 확정합니다.
2. Critical/High 우선순위 항목부터 작업을 시작합니다.
3. 변경 사항 적용 후 동일한 테스트를 재실행합니다.
4. 개선 여부를 확인하고 문서를 업데이트합니다.

## 5. 히스토리

이 문서는 자동으로 생성되었습니다. 이전 결과와 비교하려면 `reports/history/` 디렉토리를 확인하세요.

---
*Generated by feedback-loop.py*
"""
        
        return doc
    
    def compare_with_baseline(self, current: Dict, baseline_scenario: str = "baseline") -> Dict:
        """Compare current results with baseline"""
        baseline_file = self.history_dir / "baseline_reference.json"
        
        if not baseline_file.exists():
            # Save current baseline as reference
            if baseline_scenario in current:
                with open(baseline_file, "w") as f:
                    json.dump(current[baseline_scenario], f, indent=2)
                return {"message": "Baseline reference created"}
        
        with open(baseline_file) as f:
            baseline = json.load(f)
        
        current_data = current.get(baseline_scenario, {}).get("summary", {})
        baseline_data = baseline.get("summary", {})
        
        comparison = {
            "p95_change": self._calc_change(
                current_data.get("p95_duration_ms", 0),
                baseline_data.get("p95_duration_ms", 1)
            ),
            "error_rate_change": self._calc_change(
                current_data.get("error_rate", 0),
                baseline_data.get("error_rate", 0.001)
            ),
            "throughput_change": self._calc_change(
                current_data.get("requests_per_second", 0),
                baseline_data.get("requests_per_second", 1)
            ),
        }
        
        return comparison
    
    def _calc_change(self, current: float, baseline: float) -> Dict:
        """Calculate percentage change"""
        if baseline == 0:
            return {"value": 0, "direction": "unchanged"}
        
        change = ((current - baseline) / baseline) * 100
        direction = "improved" if change < 0 else "degraded" if change > 0 else "unchanged"
        
        return {
            "value": abs(change),
            "direction": direction,
            "current": current,
            "baseline": baseline,
        }
    
    def run(self, output_dir: Optional[str] = None) -> Dict:
        """Run the complete feedback loop"""
        output_dir = Path(output_dir) if output_dir else self.latest_dir
        output_dir.mkdir(parents=True, exist_ok=True)
        
        # Load results
        results = self.load_latest_results()
        
        if not results:
            return {"error": "No test results found"}
        
        # Analyze results
        analysis = self.analyze_results(results)
        
        # Generate action items
        action_items = self.generate_action_items(analysis)
        
        # Generate improvements document
        improvements_doc = self.generate_improvements_document(analysis, action_items)
        
        # Compare with baseline
        comparison = self.compare_with_baseline(results)
        
        # Save outputs
        with open(output_dir / "analysis.json", "w") as f:
            json.dump(analysis, f, indent=2, ensure_ascii=False)
        
        with open(output_dir / "action_items.json", "w") as f:
            json.dump(action_items, f, indent=2, ensure_ascii=False)
        
        with open(output_dir / "improvements.md", "w") as f:
            f.write(improvements_doc)
        
        with open(output_dir / "baseline_comparison.json", "w") as f:
            json.dump(comparison, f, indent=2, ensure_ascii=False)
        
        print(f"✅ Feedback loop completed")
        print(f"   - Analysis: {output_dir / 'analysis.json'}")
        print(f"   - Action Items: {output_dir / 'action_items.json'}")
        print(f"   - Improvements: {output_dir / 'improvements.md'}")
        
        return {
            "analysis": analysis,
            "action_items": action_items,
            "comparison": comparison,
        }


def main():
    parser = argparse.ArgumentParser(description="Run feedback loop on test results")
    parser.add_argument("--project-dir", default=".",
                       help="Project directory")
    parser.add_argument("--output-dir", 
                       help="Output directory (default: reports/latest)")
    parser.add_argument("--json", action="store_true",
                       help="Output results as JSON")
    
    args = parser.parse_args()
    
    # Resolve project directory
    script_dir = Path(__file__).parent
    project_dir = script_dir.parent if args.project_dir == "." else Path(args.project_dir)
    
    feedback = FeedbackLoop(str(project_dir))
    result = feedback.run(args.output_dir)
    
    if args.json:
        print(json.dumps(result, indent=2, ensure_ascii=False))


if __name__ == "__main__":
    main()

