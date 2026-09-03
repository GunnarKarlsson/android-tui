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