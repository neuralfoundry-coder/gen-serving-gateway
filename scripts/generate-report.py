#!/usr/bin/env python3
"""
HTML Dashboard Generator
Generates interactive HTML reports from test results
"""

import json
import os
from datetime import datetime
from pathlib import Path
from typing import Dict, Any
import argparse


def generate_html_dashboard(data: Dict[str, Any]) -> str:
    """Generate HTML dashboard from test data"""
    
    scenarios_html = ""
    for scenario, info in data.get("scenarios", {}).items():
        status_class = "healthy" if info["status"] == "healthy" else "degraded"
        metrics = info.get("metrics", {})
        
        recommendations_html = ""
        if info.get("recommendations"):
            recommendations_html = "<ul class='recommendations'>"
            for rec in info["recommendations"]:
                recommendations_html += f"<li>{rec}</li>"
            recommendations_html += "</ul>"
        
        scenarios_html += f"""
        <div class="scenario-card {status_class}">
            <h3>{scenario.upper()}</h3>
            <div class="status-badge">{info['status']}</div>
            <div class="metrics">
                <div class="metric">
                    <span class="label">평균 응답</span>
                    <span class="value">{metrics.get('avg_response_time', 0):.0f}ms</span>
                </div>
                <div class="metric">
                    <span class="label">P95 응답</span>
                    <span class="value">{metrics.get('p95_response_time', 0):.0f}ms</span>
                </div>
                <div class="metric">
                    <span class="label">에러율</span>
                    <span class="value">{metrics.get('error_rate', 0):.2f}%</span>
                </div>
                <div class="metric">
                    <span class="label">처리량</span>
                    <span class="value">{metrics.get('throughput', 0):.1f} req/s</span>
                </div>
                <div class="metric">
                    <span class="label">총 요청</span>
                    <span class="value">{metrics.get('total_requests', 0):,}</span>
                </div>
            </div>
            {recommendations_html}
        </div>
        """
    
    # Prepare chart data
    response_times = data.get("charts", {}).get("response_times", [])
    error_rates = data.get("charts", {}).get("error_rates", [])
    throughput = data.get("charts", {}).get("throughput", [])
    
    response_times_json = json.dumps(response_times)
    error_rates_json = json.dumps(error_rates)
    throughput_json = json.dumps(throughput)
    
    summary = data.get("summary", {})
    generated_at = data.get("generated_at", datetime.now().isoformat())
    
    html = f"""
<!DOCTYPE html>
<html lang="ko">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>부하 테스트 대시보드</title>
    <script src="https://cdn.jsdelivr.net/npm/chart.js"></script>
    <style>
        :root {{
            --bg-dark: #0d1117;
            --bg-card: #161b22;
            --border: #30363d;
            --text: #c9d1d9;
            --text-muted: #8b949e;
            --accent: #58a6ff;
            --success: #3fb950;
            --warning: #d29922;
            --danger: #f85149;
        }}
        
        * {{
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }}
        
        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', 'Noto Sans', Helvetica, Arial, sans-serif;
            background: var(--bg-dark);
            color: var(--text);
            line-height: 1.6;
            padding: 20px;
        }}
        
        .container {{
            max-width: 1400px;
            margin: 0 auto;
        }}
        
        header {{
            text-align: center;
            margin-bottom: 40px;
            padding: 20px;
            background: var(--bg-card);
            border-radius: 12px;
            border: 1px solid var(--border);
        }}
        
        h1 {{
            font-size: 2.5em;
            margin-bottom: 10px;
            background: linear-gradient(135deg, var(--accent), #a371f7);
            -webkit-background-clip: text;
            -webkit-text-fill-color: transparent;
        }}
        
        .timestamp {{
            color: var(--text-muted);
            font-size: 0.9em;
        }}
        
        .summary {{
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
            gap: 20px;
            margin-bottom: 40px;
        }}
        
        .summary-card {{
            background: var(--bg-card);
            padding: 20px;
            border-radius: 12px;
            border: 1px solid var(--border);
            text-align: center;
        }}
        
        .summary-card .number {{
            font-size: 3em;
            font-weight: bold;
            color: var(--accent);
        }}
        
        .summary-card.healthy .number {{
            color: var(--success);
        }}
        
        .summary-card.degraded .number {{
            color: var(--danger);
        }}
        
        .scenarios {{
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
            gap: 20px;
            margin-bottom: 40px;
        }}
        
        .scenario-card {{
            background: var(--bg-card);
            padding: 20px;
            border-radius: 12px;
            border: 1px solid var(--border);
            position: relative;
        }}
        
        .scenario-card.healthy {{
            border-left: 4px solid var(--success);
        }}
        
        .scenario-card.degraded {{
            border-left: 4px solid var(--danger);
        }}
        
        .scenario-card h3 {{
            margin-bottom: 10px;
            color: var(--accent);
        }}
        
        .status-badge {{
            position: absolute;
            top: 20px;
            right: 20px;
            padding: 4px 12px;
            border-radius: 20px;
            font-size: 0.8em;
            font-weight: bold;
            text-transform: uppercase;
        }}
        
        .healthy .status-badge {{
            background: rgba(63, 185, 80, 0.2);
            color: var(--success);
        }}
        
        .degraded .status-badge {{
            background: rgba(248, 81, 73, 0.2);
            color: var(--danger);
        }}
        
        .metrics {{
            display: grid;
            grid-template-columns: repeat(2, 1fr);
            gap: 10px;
            margin-top: 15px;
        }}
        
        .metric {{
            padding: 10px;
            background: var(--bg-dark);
            border-radius: 8px;
        }}
        
        .metric .label {{
            display: block;
            font-size: 0.8em;
            color: var(--text-muted);
        }}
        
        .metric .value {{
            display: block;
            font-size: 1.2em;
            font-weight: bold;
            color: var(--text);
        }}
        
        .recommendations {{
            margin-top: 15px;
            padding: 10px;
            background: rgba(210, 153, 34, 0.1);
            border-radius: 8px;
            border-left: 3px solid var(--warning);
        }}
        
        .recommendations li {{
            margin-left: 20px;
            color: var(--warning);
            font-size: 0.9em;
        }}
        
        .charts {{
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(400px, 1fr));
            gap: 20px;
            margin-bottom: 40px;
        }}
        
        .chart-container {{
            background: var(--bg-card);
            padding: 20px;
            border-radius: 12px;
            border: 1px solid var(--border);
        }}
        
        .chart-container h3 {{
            margin-bottom: 15px;
            color: var(--accent);
        }}
        
        footer {{
            text-align: center;
            padding: 20px;
            color: var(--text-muted);
            font-size: 0.9em;
        }}
    </style>
</head>
<body>
    <div class="container">
        <header>
            <h1>🚀 부하 테스트 대시보드</h1>
            <p class="timestamp">생성 시간: {generated_at}</p>
        </header>
        
        <section class="summary">
            <div class="summary-card">
                <div class="number">{summary.get('total_scenarios', 0)}</div>
                <div>총 시나리오</div>
            </div>
            <div class="summary-card healthy">
                <div class="number">{summary.get('healthy', 0)}</div>
                <div>정상</div>
            </div>
            <div class="summary-card degraded">
                <div class="number">{summary.get('degraded', 0)}</div>
                <div>주의 필요</div>
            </div>
        </section>
        
        <section class="scenarios">
            {scenarios_html}
        </section>
        
        <section class="charts">
            <div class="chart-container">
                <h3>📊 응답 시간 비교</h3>
                <canvas id="responseTimeChart"></canvas>
            </div>
            <div class="chart-container">
                <h3>⚠️ 에러율</h3>
                <canvas id="errorRateChart"></canvas>
            </div>
            <div class="chart-container">
                <h3>⚡ 처리량 (req/s)</h3>
                <canvas id="throughputChart"></canvas>
            </div>
        </section>
        
        <footer>
            <p>Generative Image Serving Framework - Load Test Dashboard</p>
        </footer>
    </div>
    
    <script>
        const responseTimesData = {response_times_json};
        const errorRatesData = {error_rates_json};
        const throughputData = {throughput_json};
        
        // Response Time Chart
        new Chart(document.getElementById('responseTimeChart'), {{
            type: 'bar',
            data: {{
                labels: responseTimesData.map(d => d.scenario),
                datasets: [
                    {{
                        label: 'Average',
                        data: responseTimesData.map(d => d.avg),
                        backgroundColor: 'rgba(88, 166, 255, 0.7)',
                    }},
                    {{
                        label: 'P95',
                        data: responseTimesData.map(d => d.p95),
                        backgroundColor: 'rgba(163, 113, 247, 0.7)',
                    }},
                    {{
                        label: 'P99',
                        data: responseTimesData.map(d => d.p99),
                        backgroundColor: 'rgba(248, 81, 73, 0.7)',
                    }}
                ]
            }},
            options: {{
                responsive: true,
                plugins: {{
                    legend: {{
                        labels: {{ color: '#c9d1d9' }}
                    }}
                }},
                scales: {{
                    x: {{
                        ticks: {{ color: '#8b949e' }},
                        grid: {{ color: '#30363d' }}
                    }},
                    y: {{
                        ticks: {{ color: '#8b949e' }},
                        grid: {{ color: '#30363d' }},
                        title: {{
                            display: true,
                            text: 'ms',
                            color: '#8b949e'
                        }}
                    }}
                }}
            }}
        }});
        
        // Error Rate Chart
        new Chart(document.getElementById('errorRateChart'), {{
            type: 'bar',
            data: {{
                labels: errorRatesData.map(d => d.scenario),
                datasets: [{{
                    label: 'Error Rate (%)',
                    data: errorRatesData.map(d => d.rate),
                    backgroundColor: errorRatesData.map(d => 
                        d.rate > 5 ? 'rgba(248, 81, 73, 0.7)' : 
                        d.rate > 1 ? 'rgba(210, 153, 34, 0.7)' : 
                        'rgba(63, 185, 80, 0.7)'
                    ),
                }}]
            }},
            options: {{
                responsive: true,
                plugins: {{
                    legend: {{
                        labels: {{ color: '#c9d1d9' }}
                    }}
                }},
                scales: {{
                    x: {{
                        ticks: {{ color: '#8b949e' }},
                        grid: {{ color: '#30363d' }}
                    }},
                    y: {{
                        ticks: {{ color: '#8b949e' }},
                        grid: {{ color: '#30363d' }},
                        title: {{
                            display: true,
                            text: '%',
                            color: '#8b949e'
                        }}
                    }}
                }}
            }}
        }});
        
        // Throughput Chart
        new Chart(document.getElementById('throughputChart'), {{
            type: 'bar',
            data: {{
                labels: throughputData.map(d => d.scenario),
                datasets: [{{
                    label: 'Requests/sec',
                    data: throughputData.map(d => d.rps),
                    backgroundColor: 'rgba(63, 185, 80, 0.7)',
                }}]
            }},
            options: {{
                responsive: true,
                plugins: {{
                    legend: {{
                        labels: {{ color: '#c9d1d9' }}
                    }}
                }},
                scales: {{
                    x: {{
                        ticks: {{ color: '#8b949e' }},
                        grid: {{ color: '#30363d' }}
                    }},
                    y: {{
                        ticks: {{ color: '#8b949e' }},
                        grid: {{ color: '#30363d' }},
                        title: {{
                            display: true,
                            text: 'req/s',
                            color: '#8b949e'
                        }}
                    }}
                }}
            }}
        }});
    </script>
</body>
</html>
"""
    return html


def main():
    parser = argparse.ArgumentParser(description="Generate HTML dashboard")
    parser.add_argument("--reports-dir", default="reports",
                       help="Reports directory")
    parser.add_argument("--output", "-o", default="reports/latest/dashboard.html",
                       help="Output HTML file")
    parser.add_argument("--data-file", help="Input JSON data file (optional)")
    
    args = parser.parse_args()
    
    script_dir = Path(__file__).parent
    project_dir = script_dir.parent
    
    if args.data_file:
        with open(args.data_file) as f:
            data = json.load(f)
    else:
        # Generate data using analyzer
        import sys
        sys.path.insert(0, str(script_dir))
        from analyze_results import TestResultAnalyzer
        
        reports_dir = project_dir / args.reports_dir
        analyzer = TestResultAnalyzer(str(reports_dir))
        data = analyzer.to_dashboard_data()
    
    html = generate_html_dashboard(data)
    
    output_path = project_dir / args.output
    output_path.parent.mkdir(parents=True, exist_ok=True)
    
    with open(output_path, "w") as f:
        f.write(html)
    
    print(f"Dashboard generated: {output_path}")


if __name__ == "__main__":
    main()

