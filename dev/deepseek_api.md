  -H "Authorization: Bearer ${DEEPSEEK_API_KEY}" \
  -d '{
        "model": "${DEEPSEEK_MODEL}",
        "messages": [
          {"role": "system", "content": "You are a helpful assistant."},
          {"role": "user", "content": "Hello!"}
        ],
        "thinking": {"type": "enabled"},
        "reasoning_effort": "high",
        "stream": false
      }'

example response:

2026-09-03T04:57:36.859058Z  INFO ai_insight::client: received insight response status=200 body={"id":"5272458c-695b-4413-be79-24e10cbd0478","object":"chat.completion","created":1788411454,"model":"deepseek-v4-pro","choices":[{"index":0,"message":{"role":"assistant","content":"FAILING  \nTop issues: Binder transaction failures due to “No space left on device” (14 occurrences across IPCThreadState, JavaBinder, ConnectivityService). Likely cause: storage full, blocking binder IPC and network callbacks.  \nNext checks:  \n- Verify free space on /data and /system.  \n- Identify large files or logs consuming storage.  \n- Check if ART service or package manager operations are failing due to storage."},"logprobs":null,"finish_reason":"stop"}],"usage":{"prompt_tokens":1209,"completion_tokens":91,"total_tokens":1300,"prompt_tokens_details":{"cached_tokens":0},"prompt_cache_hit_tokens":0,"prompt_cache_miss_tokens":1209},"system_fingerprint":"a307abda487cd1b463329ccb945ce396"}