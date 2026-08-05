from __future__ import annotations
import argparse,json,sys
from pathlib import Path
from .controller import LifecycleError,load_manifest,replay,run_lifecycle
def emit(value):print(json.dumps(value,sort_keys=True,separators=(",",":")))
def main(argv=None):
 parser=argparse.ArgumentParser(prog="mfw-autonomic-lifecycle");sub=parser.add_subparsers(dest="command",required=True)
 check=sub.add_parser("check");check.add_argument("manifest")
 run=sub.add_parser("run");run.add_argument("manifest");run.add_argument("--receipt");run.add_argument("--no-repair",action="store_true")
 rep=sub.add_parser("replay");rep.add_argument("manifest");rep.add_argument("receipt");args=parser.parse_args(argv)
 try:
  if args.command=="check":
   value=load_manifest(args.manifest);emit({"ok":True,"manifest_id":value["id"],"stages":len(value["stages"])});return 0
  if args.command=="run":
   receipt=run_lifecycle(args.manifest,not args.no_repair)
   if args.receipt:Path(args.receipt).write_text(json.dumps(receipt,sort_keys=True,separators=(",",":"))+"\n",encoding="utf-8")
   emit(receipt);return 0 if receipt["standing"]=="ALIVE" else 3
  prior=json.loads(Path(args.receipt).read_text(encoding="utf-8"));result=replay(args.manifest,prior);emit(result);return 0 if result["agreement"] else 4
 except LifecycleError as error:emit({"ok":False,"standing":"REFUSED","error":{"code":error.code,"details":error.details}});return 2
 except (json.JSONDecodeError,FileNotFoundError) as error:emit({"ok":False,"standing":"REFUSED","error":{"code":"RECEIPT_INPUT_INVALID","details":{"reason":str(error)}}});return 2
if __name__=="__main__":sys.exit(main())
