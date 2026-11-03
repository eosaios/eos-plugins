#!/usr/bin/python3
import json,sys
for line in sys.stdin:
    request=json.loads(line)
    if 'id' not in request:continue
    method=request.get('method')
    if method=='initialize':result={'protocolVersion':'2025-11-25','capabilities':{'tools':{}},'serverInfo':{'name':'isolated-fixture','version':'1.0.0'}}
    elif method=='tools/list':result={'tools':[{'name':'echo','description':'Echo fixture input','inputSchema':{'type':'object'}}]}
    elif method=='tools/call':result={'content':[{'type':'text','text':'echo:'+str(request.get('params',{}).get('arguments',{}).get('text',''))}]}
    else:result={}
    print(json.dumps({'jsonrpc':'2.0','id':request['id'],'result':result}),flush=True)
