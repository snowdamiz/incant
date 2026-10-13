import concurrent.futures,gzip,hashlib,json,pathlib,subprocess
p=pathlib.Path(__file__).parent
jobs={'macos-checks':115162102607,'linux-desktop':115161976443,'windows-desktop':115161976526,'windows-optimized':115161976221}
def one(item):
 name,job=item
 q=subprocess.run(['gh','api','--allow-escape-sequences',f'repos/snowdamiz/incant/actions/jobs/{job}/logs'],capture_output=True,check=True)
 raw=q.stdout; (p/(name+'.log')).write_bytes(raw); zipped=gzip.compress(raw,mtime=0);(p/(name+'.log.gz')).write_bytes(zipped)
 return name,{'job':job,'raw_sha256':hashlib.sha256(raw).hexdigest(),'raw_bytes':len(raw),'gzip_sha256':hashlib.sha256(zipped).hexdigest(),'gzip_bytes':len(zipped)}
with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool: results=dict(pool.map(one,jobs.items()))
(p/'completed-job-logs.json').write_text(json.dumps(results,indent=2)+'\n')
print(json.dumps(results))
