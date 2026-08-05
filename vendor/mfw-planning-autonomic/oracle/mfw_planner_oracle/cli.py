"""JSON CLI for the vendored MFW planner oracle capsule."""
from __future__ import annotations
import argparse,json,sys
from pathlib import Path
from .compare import compare
from .oracle import OracleError,PLANNING_TYPES,solve_json
def _read(path):return sys.stdin.read() if path in (None,"-") else Path(path).read_text(encoding="utf-8")
def main(argv=None):
 parser=argparse.ArgumentParser(prog="mfw-planner-oracle");sub=parser.add_subparsers(dest="command",required=True)
 solve_parser=sub.add_parser("solve");solve_parser.add_argument("request",nargs="?",default="-")
 compare_parser=sub.add_parser("compare");compare_parser.add_argument("request");compare_parser.add_argument("candidate")
 validate_parser=sub.add_parser("validate");validate_parser.add_argument("result",nargs="?",default="-")
 sub.add_parser("capabilities");args=parser.parse_args(argv)
 if args.command=="capabilities":print(json.dumps({"oracle":"mfw-python-v1","planning_types":PLANNING_TYPES},sort_keys=True));return 0
 if args.command=="solve":
  output=solve_json(_read(args.request));print(output);return 0 if json.loads(output).get("ok") else 2
 if args.command=="compare":
  try:
   receipt=compare(json.loads(_read(args.request)),json.loads(_read(args.candidate)));print(json.dumps(receipt,sort_keys=True,separators=(",",":")));return 0 if receipt["agreement"] else 3
  except (json.JSONDecodeError,OracleError) as error:
   code=error.code if isinstance(error,OracleError) else "INVALID_JSON";print(json.dumps({"ok":False,"error":{"code":code}},sort_keys=True));return 2
 try:
  value=json.loads(_read(args.result));valid=isinstance(value,dict) and value.get("ok") is True and value.get("oracle")=="mfw-python-v1" and isinstance(value.get("result"),dict) and value["result"].get("planning_type") in PLANNING_TYPES
 except json.JSONDecodeError:valid=False
 print(json.dumps({"valid":valid},sort_keys=True));return 0 if valid else 2
if __name__=="__main__":raise SystemExit(main())
