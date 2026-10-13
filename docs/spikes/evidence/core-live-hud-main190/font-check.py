from pathlib import Path
import sys,json,hashlib
root=Path.cwd();sys.path.insert(0,str(root/'samples/core'))
import hud
from art import fontcheck
from scene import FONT_SOURCES
out=Path(sys.argv[1]);texts=json.loads((out/'native/localized.json').read_text());chain=[(k,Path(p).read_bytes()) for k,p in FONT_SOURCES.items()]
result={}
for locale,table in texts.items():
 supplied,missing=fontcheck.coverage(hud.display_strings(table),chain)
 assert not missing,(locale,missing)
 result[locale]={'coverage':supplied,'missing':missing}
(out/'font-coverage.json').write_text(json.dumps({'fonts':{k:hashlib.sha256(data).hexdigest() for k,data in chain},'locales':result},indent=2)+'\n')
print(json.dumps({'locales':list(result),'missing':0}))
