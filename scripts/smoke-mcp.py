"""Exercise the real stdio executable with isolated local data, no Agent runtime needed."""
import json, os, pathlib, subprocess, sys, tempfile
repo=pathlib.Path(__file__).resolve().parents[1]
binary=pathlib.Path(sys.argv[1]).resolve() if len(sys.argv)>1 else repo/'src-tauri'/'target'/'debug'/('subbit.exe' if os.name=='nt' else 'subbit')
record={'id':'smoke-1','name':'MCP smoke example','serviceId':None,'category':'other','plan':'Annual','icon':None,'amount':'1200','currency':'CNY','kind':'recurring','cycleUnit':'year','cycleCount':1,'purchasedAt':'2025-01-31','expiresAt':None,'autoRenew':True,'active':True,'reimbursement':'fixed','reimbursementValue':'600','paymentMethod':'card','notes':'Test fixture','fixedUsdRate':None}
with tempfile.TemporaryDirectory(prefix='subbit-mcp-') as root:
 data=pathlib.Path(root)/'data';data.mkdir()
 (data/'subscriptions.json').write_text(json.dumps({'schemaVersion':1,'revision':0,'subscriptions':[],'settings':{'mcpEnabled':True,'autoRates':False},'rates':{},'imports':{}}))
 p=subprocess.Popen([str(binary),'--mcp'],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True,encoding='utf-8',env={**os.environ,'SUBBIT_CONFIG_DIR':root})
 def request(id,method,params=None):
  p.stdin.write(json.dumps({'jsonrpc':'2.0','id':id,'method':method,'params':params or {}})+'\n');p.stdin.flush()
  response=json.loads(p.stdout.readline());assert response['id']==id and 'error' not in response,response
  return response['result']
 def call(id,name,args=None):
  result=request(id,'tools/call',{'name':name,'arguments':args or {}});assert not result.get('isError'),result
  return json.loads(result['content'][0]['text'])
 initialized=request(1,'initialize',{'protocolVersion':'2025-11-25','clientInfo':{'name':'smoke','version':'1'},'capabilities':{}})
 assert initialized['protocolVersion']=='2025-11-25'
 assert initialized['serverInfo']['version']==json.loads((repo/'package.json').read_text(encoding='utf-8'))['version']
 p.stdin.write('{"jsonrpc":"2.0","method":"notifications/initialized"}\n');p.stdin.flush()
 assert len(request(2,'tools/list')['tools'])==5
 catalog=call(3,'get_catalog')['catalog']
 expected_services=[json.loads(path.read_text(encoding='utf-8')) for path in sorted((repo/'catalog'/'services').glob('*.json'))]
 assert catalog['services']==expected_services
 assert catalog['categories']==json.loads((repo/'catalog'/'categories.json').read_text(encoding='utf-8'))
 assert call(4,'preview_import',{'records':[record]})['added']==1
 assert call(5,'list_subscriptions')['subscriptions']==[]
 assert call(6,'import_subscriptions',{'importId':'test-import','records':[record]})['added']==1
 assert call(7,'import_subscriptions',{'importId':'test-import','records':[record]})['replayed']
 assert len(call(8,'list_subscriptions')['subscriptions'])==1
 total=call(9,'get_summary',{'currency':'CNY','period':'month'})
 assert total['bill']=='100' and total['actual']=='50',total
 p.stdin.close();p.wait(timeout=10);assert p.returncode==0,p.stderr.read()
 assert json.loads((data/'subscriptions.json').read_text(encoding='utf-8'))['subscriptions'][0]['paymentMethod']=='card'
 assert (data/'subscriptions.json.bak').exists()
 print('MCP smoke passed: initialize, catalog, preview, import, retry, list, summary, persisted backup.')
