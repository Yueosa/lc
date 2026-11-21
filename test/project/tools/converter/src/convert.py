import json
import sys


for line in sys.stdin:
    event = json.loads(line)
    print(json.dumps({"device_id": event["device"], "reading": event["value"]}))
