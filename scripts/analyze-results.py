#!/usr/bin/env python3
"""
Test Results Analyzer
Analyzes k6 load test results and generates reports
"""

import json
import os
import sys
from datetime import datetime
from pathlib import Path
from typing import Dict, List, Any, Optional
import argparse


class TestResultAnalyzer:
    """Analyzes test results and generates insights"""
    
    def __init__(self, reports_dir: str):
        self.reports_dir = Path(reports_dir)
        self.latest_dir = self.reports_dir / "latest"
        self.history_dir = self.reports_dir / "history"
        
    def load_results(self, scenario: str) -> Optional[Dict]:
        """Load results for a specific scenario"""
        result_file = self.latest_dir / f"{scenario}.json"
        if result_file.exists():
            with open(result_file) as f:
                return json.load(f)
        return None
    
    def load_all_latest_results(self) -> Dict[str, Dict]:
        """Load all latest test results"""
        results = {}
        scenarios = ["baseline", "spike", "stress", "soak", "breakpoint"]
        
        for scenario in scenarios:
            data = self.load_results(scenario)
            if data:
                results[scenario] = data
        
        return results
    
    def load_history(self, scenario: str, limit: int = 10) -> List[Dict]:
        """Load historical results for a scenario"""
        history = []
        
        if not self.history_dir.exists():
            return history
        
        # Get all history directories sorted by name (timestamp)
        history_dirs = sorted(self.history_dir.iterdir(), reverse=True)[:limit]
        
        for hist_dir in history_dirs:
            result_file = hist_dir / f"{scenario}.json"
            if result_file.exists():
                with open(result_file) as f:
                    data = json.load(f)
                    data["_history_timestamp"] = hist_dir.name
                    history.append(data)
        
        return history
    
    def analyze_performance_trends(self, scenario: str) -> Dict:
        """Analyze performance trends over time"""
        history = self.load_history(scenario)
        current = self.load_results(scenario)
        
        if not current:
            return {"error": f"No current results for {scenario}"}
        
        analysis = {
            "scenario": scenario,
            "current": {
                "timestamp": current.get("timestamp"),
                "avg_duration": current.get("summary", {}).get("avg_duration_ms"),
                "p95_duration": current.get("summary", {}).get("p95_duration_ms"),
                "error_rate": current.get("summary", {}).get("error_rate"),
            },
            "trends": [],
            "recommendations": [],
        }
        
        if history:
            # Calculate trends
            p95_values = [h.get("summary", {}).get("p95_duration_ms", 0) for h in history]
            error_rates = [h.get("summary", {}).get("error_rate", 0) for h in history]
            
            if p95_values:
                avg_p95 = sum(p95_values) / len(p95_values)
                current_p95 = analysis["current"]["p95_duration"] or 0
                
                if current_p95 > avg_p95 * 1.2:
                    analysis["trends"].append({
                        "metric": "p95_duration",
                        "direction": "degraded",
                        "change": f"+{((current_p95 / avg_p95) - 1) * 100:.1f}%",
                        "baseline": f"{avg_p95:.0f}ms",
                    })
                    analysis["recommendations"].append(
                        "P95 latency has increased significantly. Investigate recent changes."
                    )
                elif current_p95 < avg_p95 * 0.8:
                    analysis["trends"].append({
                        "metric": "p95_duration",
                        "direction": "improved",
                        "change": f"{((1 - current_p95 / avg_p95)) * 100:.1f}%",
                    })
            
            if error_rates:
                avg_error = sum(error_rates) / len(error_rates)
                current_error = analysis["current"]["error_rate"] or 0
                
                if current_error > avg_error * 2 and current_error > 0.01:
                    analysis["trends"].append({
                        "metric": "error_rate",
                        "direction": "degraded",
                        "change": f"+{((current_error / max(avg_error, 0.001)) - 1) * 100:.1f}%",
                    })
                    analysis["recommendations"].append(
                        "Error rate has increased. Check backend health and logs."
                    )
        
        return analysis
    
    def generate_improvements_doc(self) -> str:
        """Generate improvement recommendations document"""
        results = self.load_all_latest_results()
        
        doc = f"""# 테스트 결과 분석 및 개선사항
생성 시간: {datetime.now().isoformat()}

## 요약

"""
        
        # Overall status
        total_scenarios = len(results)
        passed = sum(1 for r in results.values() 
                    if r.get("analysis", {}).get("system_stable", True))
        
        doc += f"- 실행된 시나리오: {total_scenarios}\n"
        doc += f"- 안정적인 시나리오: {passed}/{total_scenarios}\n\n"
        
        # Per-scenario analysis
        doc += "## 시나리오별 분석\n\n"
        
        for scenario, data in results.items():
            doc += f"### {scenario.upper()}\n\n"
            
            summary = data.get("summary", {})
            analysis = data.get("analysis", {})
            
            doc += f"| 메트릭 | 값 |\n"
            doc += f"|--------|----|\n"
            doc += f"| 총 요청 수 | {summary.get('total_requests', 'N/A'):,} |\n"
            doc += f"| 평균 응답 시간 | {summary.get('avg_duration_ms', 'N/A'):.0f}ms |\n"
            doc += f"| P95 응답 시간 | {summary.get('p95_duration_ms', 'N/A'):.0f}ms |\n"
            doc += f"| 에러율 | {summary.get('error_rate', 0) * 100:.2f}% |\n\n"
            
            # Recommendations
            recommendations = analysis.get("recommendations", [])
            if recommendations:
                doc += "#### 권장 조치사항\n\n"
                for rec in recommendations:
                    doc += f"- {rec}\n"
                doc += "\n"
            
            # Breaking point info
            if scenario == "breakpoint":
                bp = analysis.get("breaking_point", "N/A")
                doc += f"#### 성능 한계\n\n"
                doc += f"- 추정 한계점: {bp}\n"
                if "capacity_planning" in analysis:
                    cp = analysis["capacity_planning"]
                    doc += f"- 권장 최대 부하: {cp.get('recommended_max_load', 'N/A')}\n"
                    doc += f"- 스케일링 트리거: {cp.get('scale_trigger', 'N/A')}\n"
                doc += "\n"
        
        # Action items
        doc += "## 우선순위 조치사항\n\n"
        
        all_recommendations = []
        for scenario, data in results.items():
            for rec in data.get("analysis", {}).get("recommendations", []):
                all_recommendations.append(f"[{scenario}] {rec}")
        
        if all_recommendations:
            for i, rec in enumerate(all_recommendations, 1):
                doc += f"{i}. {rec}\n"
        else:
            doc += "현재 특별한 조치사항이 없습니다. ✅\n"
        
        return doc
    
    def to_dashboard_data(self) -> Dict:
        """Convert results to dashboard-compatible format"""
        results = self.load_all_latest_results()
        
        dashboard_data = {
            "generated_at": datetime.now().isoformat(),
            "scenarios": {},
            "summary": {
                "total_scenarios": len(results),
                "healthy": 0,
                "degraded": 0,
            },
            "charts": {
                "response_times": [],
                "error_rates": [],
                "throughput": [],
            }
        }
        
        for scenario, data in results.items():
            summary = data.get("summary", {})
            analysis = data.get("analysis", {})
            
            is_stable = analysis.get("system_stable", True)
            if is_stable:
                dashboard_data["summary"]["healthy"] += 1
            else:
                dashboard_data["summary"]["degraded"] += 1
            
            dashboard_data["scenarios"][scenario] = {
                "status": "healthy" if is_stable else "degraded",
                "metrics": {
                    "avg_response_time": summary.get("avg_duration_ms", 0),
                    "p95_response_time": summary.get("p95_duration_ms", 0),
                    "p99_response_time": summary.get("p99_duration_ms", 0),
                    "error_rate": summary.get("error_rate", 0) * 100,
                    "throughput": summary.get("requests_per_second", 0),
                    "total_requests": summary.get("total_requests", 0),
                },
                "recommendations": analysis.get("recommendations", []),
            }
            
            # Chart data
            dashboard_data["charts"]["response_times"].append({
                "scenario": scenario,
                "avg": summary.get("avg_duration_ms", 0),
                "p95": summary.get("p95_duration_ms", 0),
                "p99": summary.get("p99_duration_ms", 0),
            })
            
            dashboard_data["charts"]["error_rates"].append({
                "scenario": scenario,
                "rate": summary.get("error_rate", 0) * 100,
            })
            
            dashboard_data["charts"]["throughput"].append({
                "scenario": scenario,
                "rps": summary.get("requests_per_second", 0),
            })
        
        return dashboard_data


def main():
    parser = argparse.ArgumentParser(description="Analyze test results")
    parser.add_argument("--reports-dir", default="reports",
                       help="Reports directory")
    parser.add_argument("--generate-report", action="store_true",
                       help="Generate improvement document")
    parser.add_argument("--generate-dashboard-data", action="store_true",
                       help="Generate dashboard JSON data")
    parser.add_argument("--analyze", metavar="SCENARIO",
                       help="Analyze specific scenario")
    parser.add_argument("--output", "-o", help="Output file")
    
    args = parser.parse_args()
    
    # Get project root
    script_dir = Path(__file__).parent
    project_dir = script_dir.parent
    reports_dir = project_dir / args.reports_dir
    
    analyzer = TestResultAnalyzer(str(reports_dir))
    
    if args.analyze:
        analysis = analyzer.analyze_performance_trends(args.analyze)
        output = json.dumps(analysis, indent=2, ensure_ascii=False)
    elif args.generate_report:
        output = analyzer.generate_improvements_doc()
    elif args.generate_dashboard_data:
        data = analyzer.to_dashboard_data()
        output = json.dumps(data, indent=2, ensure_ascii=False)
    else:
        # Default: show summary
        results = analyzer.load_all_latest_results()
        print(f"Loaded {len(results)} scenario results")
        for scenario, data in results.items():
            summary = data.get("summary", {})
            print(f"\n{scenario}:")
            print(f"  Requests: {summary.get('total_requests', 'N/A')}")
            print(f"  Avg Duration: {summary.get('avg_duration_ms', 'N/A')}ms")
            print(f"  Error Rate: {summary.get('error_rate', 0) * 100:.2f}%")
        return
    
    if args.output:
        with open(args.output, "w") as f:
            f.write(output)
        print(f"Output written to {args.output}")
    else:
        print(output)


if __name__ == "__main__":
    main()

