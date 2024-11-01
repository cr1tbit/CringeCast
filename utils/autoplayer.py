import requests
import random
import time

url = "http://localhost:5000/"

# Function to perform the GET request
def make_request():
    name_value = f"play/mc/cave{random.randint(1, 12)}"
    
    try:
        response = requests.get(url + name_value)
        print(f"Request to {response.url} - Status Code: {response.status_code}")
    except requests.RequestException as e:
        print(f"An error occurred: {e}")

# Run requests in a loop with random intervals
while True:
    make_request()
    next_time = time.time() + random.randint(5 * 60, 20 * 60)
    while time.time() < next_time:
        remaining_time = int (next_time - time.time())
        print(f"Next request in {remaining_time/60:2.0f}:{remaining_time%60:2.0f}", end="\r")
        time.sleep(1)
