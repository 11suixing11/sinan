"""Remove borrowed provider credentials from the exact fixed IP script."""
import hashlib
import re

MAX_SOURCE = 2 * 1024 * 1024
SOURCES = {
    'ip.sh': {
        'source_sha256': '66c7f3c662f24bdccbd142acf25d2a176fbe72ff417848ef977aa755d6d38c8e',
        'patched_sha256': 'ed9787f0baba1d2ae97485dbc88aa7edd5c8a9ef3ca752667f604585bc205e7b',
    },
}

HELPERS = r'''# Sinan modification (2026-10-01): do not borrow provider credentials.
sinan_access_not_attempted(){
jq -cn --arg provider "$1" --arg target_ip "$IP" --arg checked_at "$(date +%s)" --arg message "$2" '{provider:$provider,status:"unknown",target_ip:$target_ip,checked_at:($checked_at|tonumber),Attempted:false,last_attempt_at:null,elapsed_seconds:null,Error:{category:"credential_not_configured",phase:"not_attempted",message:$message},Attempts:[]}'
}
sinan_access_youtube_metadata(){
jq -cn --arg target_ip "$IP" --arg checked_at "$(date +%s)" --arg status "$1" --arg category "$2" --arg message "$3" --argjson attempts "${youtube[attempts]:-[]}" '{provider:"youtube-public-page",status:$status,target_ip:$target_ip,checked_at:($checked_at|tonumber),Attempted:($attempts|length>0),last_attempt_at:($attempts[-1].attempted_at//null),elapsed_seconds:($attempts[-1].elapsed_seconds//null),Error:(if $category=="" then null else {category:$category,message:$message} end),Attempts:$attempts}'
}
sinan_access_youtube_unknown(){
youtube[ustatus]='未知'
youtube[uregion]="${smedia[nodata]}"
youtube[utype]="${smedia[nodata]}"
youtube[attempts]=$(jq -cn --argjson attempts "${youtube[attempts]:-[]}" --arg category "$1" '$attempts|map(.error=$category)')
youtube[access]=$(sinan_access_youtube_metadata unknown "$1" "$2")
printf 'YouTube：未知（%s）\n' "$2"
}
sinan_access_youtube_fetch(){
local response code http elapsed body category='' reason='' attempted url
url='https://www.youtube.com/premium'
attempted=$(date +%s)
if response=$(curl $CurlARG -$1 -fsSL --max-time 10 --max-filesize 2097152 -H 'Accept-Language: en' --write-out $'\n%{http_code}\n%{time_total}' "$url" 2>/dev/null);then code=0;else code=$?;fi
elapsed=${response##*$'\n'}
response=${response%$'\n'*}
http=${response##*$'\n'}
body=${response%$'\n'*}
[[ $http =~ ^[0-9]{3}$ ]] || http=000
[[ $elapsed =~ ^[0-9]+([.][0-9]+)?$ ]] || elapsed=''
case "$http" in
403) category=http_403;reason='HTTP 403：来源拒绝请求';;
429) category=http_429;reason='HTTP 429：来源限流';;
esac
if [[ -z $category ]];then
case "$code" in
6) category=dns;reason='DNS 解析失败';;
7) category=connection;reason='连接失败';;
28) category=timeout;reason='请求超时';;
35|51|58|60|77) category=tls;reason='TLS 校验或握手失败';;
63) category=response_too_large;reason='响应超过 2 MiB 限制';;
0) if [[ $http != 200 ]];then category=http_status;reason="非预期 HTTP $http";
   elif [[ -z ${body//[[:space:]]/} ]];then category=empty_response;reason='响应为空或仅有空白';
   elif [[ $(LC_ALL=C printf '%s' "$body"|wc -c) -gt 2097152 ]];then category=response_too_large;reason='响应超过 2 MiB 限制';fi;;
*) category=transport;reason="请求失败（curl $code）";;
esac
fi
youtube[attempts]=$(jq -cn --arg target_ip "$IP" --arg url "$url" --arg attempted_at "$attempted" --arg elapsed_seconds "$elapsed" --arg http_status "$http" --argjson curl_exit "$code" --arg error "$category" '[{target_ip:$target_ip,url:$url,attempted_at:($attempted_at|tonumber),elapsed_seconds:(if $elapsed_seconds=="" then null else ($elapsed_seconds|tonumber) end),http_status:(if $http_status=="000" then null else ($http_status|tonumber) end),curl_exit:$curl_exit,error:(if $error=="" then null else $error end)}]')
if [[ -n $category ]];then sinan_access_youtube_unknown "$category" "$reason";return 1;fi
printf -v "$2" '%s' "$body"
}
'''.encode()

FUNCTION_REPLACEMENTS = {
    'db_ipregistry': r'''db_ipregistry(){
ipregistry=()
ipregistry[susetype]='未知'
ipregistry[scomtype]='未知'
ipregistry[access]=$(sinan_access_not_attempted ipregistry '未配置经授权的正式接口；未查询该来源，信息未知')
printf '%s\n' 'ipregistry：未知（未配置经授权的正式接口，未查询该来源）'
}
'''.encode(),
    'db_dbip': r'''db_dbip(){
dbip=()
dbip[risk]='未知'
dbip[score]=''
dbip[access]=$(sinan_access_not_attempted DBIP '未配置经授权的正式接口；未查询该来源，信息未知')
printf '%s\n' 'DB-IP：未知（未配置经授权的正式接口，未查询该来源）'
}
'''.encode(),
    'MediaUnlockTest_DisneyPlus': r'''function MediaUnlockTest_DisneyPlus(){
disney=()
disney[ustatus]='未知'
disney[uregion]="${smedia[nodata]}"
disney[utype]="${smedia[nodata]}"
disney[access]=$(sinan_access_not_attempted disney-plus '未配置经授权的查询方式；未查询该来源，流媒体可用性未知')
printf '%s\n' 'Disney+：未知（未配置经授权的查询方式，未查询该来源）'
}
'''.encode(),
    'MediaUnlockTest_YouTube_Premium': r'''function MediaUnlockTest_YouTube_Premium(){
youtube=()
local result region lower title
sinan_access_youtube_fetch "$1" result || return 0
lower=${result,,}
title=$(printf '%s' "$lower"|grep -oE '<title>[^<]*</title>')
if [[ $lower != *'<html'* || $lower != *'</html>'* || $lower != *'<head'* || $lower != *'</head>'* || $lower != *'<body'* || $lower != *'</body>'* ||
      ( $title != '<title>youtube premium</title>' && $title != '<title>youtube premium - youtube</title>' && $title != '<title>premium - youtube</title>' ) ||
      ( $lower == *'<form'* && ( $lower == *'accounts.google.com'* || $lower == *'consent.google.com'* || $lower == *'consent.youtube.com'* ) ) ||
      $lower == *'id="challenge-form"'* || $lower == *'id="captcha-form"'* || $lower == *'class="g-recaptcha"'* ]];then
sinan_access_youtube_unknown schema_mismatch '响应不是可确认的 Premium 页面，登录或挑战页面信息未知'
return 0
fi
region=$(printf '%s' "$result"|grep -oE '"contentRegion"[[:space:]]*:[[:space:]]*"[A-Z][A-Z]"'|sed -E 's/.*"([A-Z][A-Z])"/\1/'|sort -u)
if [[ ! $region =~ ^[A-Z][A-Z]$ ]];then
sinan_access_youtube_unknown schema_mismatch '页面地区字段缺失或不一致'
return 0
fi
if [[ $region == CN && $result == *'www.google.cn'* ]];then
youtube[ustatus]="${smedia[cn]}"
youtube[uregion]="  [CN]   "
youtube[utype]="${smedia[nodata]}"
elif [[ $result == *'Premium is not available in your country'* ]];then
youtube[ustatus]="${smedia[noprem]}"
youtube[uregion]="${smedia[nodata]}"
youtube[utype]="${smedia[nodata]}"
elif [[ $result == *'ad-free'* ]];then
local result1 result3 resultunlocktype
result1=$(Check_DNS_1 www.youtube.com)
result3=$(Check_DNS_3 www.youtube.com)
resultunlocktype=$(Get_Unlock_Type "$result1" "$result3")
youtube[ustatus]="${smedia[yes]}"
youtube[uregion]="  [$region]   "
youtube[utype]="$resultunlocktype"
else
sinan_access_youtube_unknown schema_mismatch '页面未确认 Premium 可用性'
return 0
fi
# This preserves the public-page heuristic; it is not a playback proof.
youtube[access]=$(sinan_access_youtube_metadata succeeded '' '')
}
'''.encode(),
    'read_ref': r'''read_ref(){
Media_Cookie='' # Sinan: public cookies are not fetched or used.
IATA_Database="${rawgithub}main/ref/iata-icao.csv"
}
'''.encode(),
}
FUNCTIONS = tuple(FUNCTION_REPLACEMENTS)
JSON_ANCHOR = b'ipjson=$(echo "$ipjson"|jq "$head_updates$basic_updates$type_updates$score_updates$factor_updates$media_updates$mail_updates.")\n'
JSON_ADDITION = r'''ipjson=$(printf '%s' "$ipjson"|jq --argjson registry "${ipregistry[access]:-null}" --argjson dbip "${dbip[access]:-null}" --argjson disney "${disney[access]:-null}" --argjson youtube "${youtube[access]:-null}" '.Sources=(.Sources//{}) | .Sources.ipregistry=$registry | .Sources.DBIP=$dbip | .Media.DisneyPlus += $disney | .Media.Youtube += $youtube')
'''.encode()
METADATA_REPLACEMENTS = [(JSON_ANCHOR, JSON_ANCHOR + JSON_ADDITION)]


def function_span(name, content):
    if name not in FUNCTION_REPLACEMENTS or not isinstance(content, bytes):
        raise ValueError('unknown access policy function or source type')
    pattern = rb'(?m)^(?:function )?' + name.encode() + rb'\(\)\{\n[^\0]*?^\}\n'
    matches = list(re.finditer(pattern, content))
    if len(matches) != 1:
        raise ValueError('access policy requires unique complete function boundaries')
    return matches[0].span()


def replace_function(name, content, replacement):
    start, stop = function_span(name, content)
    return content[:start] + replacement + content[stop:]


def patch(content):
    for name, replacement in FUNCTION_REPLACEMENTS.items():
        content = replace_function(name, content, replacement)
    anchor = FUNCTION_REPLACEMENTS['db_ipregistry']
    if content.count(anchor) != 1:
        raise ValueError('access policy requires a unique helper insertion anchor')
    content = content.replace(anchor, HELPERS + anchor, 1)
    for before, after in METADATA_REPLACEMENTS:
        if content.count(before) != 1:
            raise ValueError('access policy requires unique JSON anchors')
        content = content.replace(before, after, 1)
    return content


def transform(role, content):
    if role not in SOURCES or not isinstance(content, bytes) or len(content) > MAX_SOURCE:
        raise ValueError('unknown access policy role or source byte limit exceeded')
    identity = SOURCES[role]
    if hashlib.sha256(content).hexdigest() != identity['source_sha256']:
        raise ValueError('access policy input SHA256 mismatch')
    result = patch(content)
    if len(result) > MAX_SOURCE + 8192 or hashlib.sha256(result).hexdigest() != identity['patched_sha256']:
        raise ValueError('access policy output SHA256 or byte limit mismatch')
    return result
