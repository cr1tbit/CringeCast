from flask import Flask, escape, request, send_from_directory, g, session
# import werkzeug
import json

import CringeCast.shell_wrappers as shell_wrappers
import os

import datetime

app = Flask(__name__,static_url_path="/static")
app.config['MAX_CONTENT_LENGTH'] = 1 * 1000 * 1000
app.super_secret_key="perystaltyka"
app.privilleged_volume = 80

'''
@app.route('/file/<filename>')
def file(filename:str):
    filename_sanitized = sanitize(filename)
    try:
        with open("text_files/" + filename_sanitized,'r') as fh:
            file_content = fh.read()
            file_content_sanitized = sanitize(file_content)
            speak(file_content_sanitized,"en")
    except:
        speak("file not found.","en")

    return f'OK'
'''

class TeapotModeHandler:

    _teapot_mode = False

    # This functionality is to prevent the cringebox from being raided,
    # or to mute it during events, or if someone is acting particularly
    # assholey.

    # If user is privilleged and knows the super_secret_key, he may
    # override this mode though. Infrastructure may do the same.

    # On 1st click, cringebox is timed out for 15mins. If after 15mins
    # The raid still persists, user may request mute again, and this time
    # the device will be muted for 4 hours.

    _unmute_at = datetime.datetime.utcfromtimestamp(0) 

    def in_teapot_mode(self):
        if datetime.datetime.now() < self._unmute_at:
            return True
        else:
            return False

    def set_teapot_mode(self,permanent=False):
        if permanent:
            self._unmute_at = datetime.datetime.now() + datetime.timedelta(days=69)

        #this means the user requested the mute 2 times in a row
        if datetime.datetime.now() - self._unmute_at < datetime.timedelta(minutes=20):
            self._unmute_at = datetime.datetime.now() + datetime.timedelta(hours=4)
        else:
            self._unmute_at = datetime.datetime.now() + datetime.timedelta(minutes=15)            

    def disable_teapot_mode(self):
        self._unmute_at = None

    def get_remaining_teapot_time(self) ->str:
        if datetime.datetime.now() < self._unmute_at:
            return str(self._unmute_at - datetime.datetime.now())
        else:
            return "0"

    
tmHandler = TeapotModeHandler()

@app.before_request
def assert_request_permissions():
    g.privilleged = False
    if request.path == "/" or request.path.split("/")[1] == "static":
        #always serve frontend stuff
        return

    if request.args.get("super_secret_key","") == app.super_secret_key:
        #if user knows the secret key, let him do anything
        g.privilleged = True
        return

    if g.privilleged and "critical" in request.args:
        shell_wrappers.set_volume(app.privilleged_volume)

    if tmHandler.in_teapot_mode() and not g.privilleged:
        return f"I'm sorry, I'm a teapot for next {tmHandler.get_remaining_teapot_time()}", 418

@app.route('/teapot/<target_state>')
def setup_teapot_mode(target_state:str):
    if target_state == "on":
        tmHandler.set_teapot_mode()
        return "Teapot mode set"
    elif target_state == "off":
        tmHandler.disable_teapot_mode()
        return "Teapot mode disabled"
    elif target_state == "permanent" and g.privilleged:
        tmHandler.set_teapot_mode(permanent=True)
        return "I'm permanently teapot now."
    else:
        return "Invalid request",500


@app.route('/favicon.png')
def serve_favicon():
    return send_from_directory('static', "favicon.png")

@app.route('/say/<saying>')
def say(saying:str):
    shell_wrappers.speak(saying,"en")
    return f'OK: {saying}'

@app.route('/mow/<saying>')
def mow(saying:str):
    shell_wrappers.speak(saying,"pl")
    return f'OK: {saying}'

@app.route('/play/<filename>')
def play(filename:str):
    shell_wrappers.play_file(filename)
    return f'OK: {filename}.mp3'
    
@app.route('/uploader', methods=['POST'])
def upload_file():
    if request.method == 'POST':
        f = request.files['file']
        f.save("audio_files/recent_upload.mp3")
        shell_wrappers.play_file("recent_upload")
        return 'file uploaded'

@app.route('/getFilelist')
def getFilelist():
    audio_names = [f.split(".")[0] \
        for f in os.listdir("audio_files")\
        if f.split(".")[1] == "mp3"]

    return json.dumps(audio_names)

@app.route('/vol/<int:vol_percent>')
def set_vol(vol_percent:int):
    if 0 <= vol_percent <= 100:
        shell_wrappers.set_volume(vol_percent)
        return f'volume set ok: {vol_percent}%'
    else:
        return f'invalid volume requested: {vol_percent}%'

@app.route('/guess/<saying>')
@app.route('/<saying>')
def speak_any_lang(saying:str):
    lang = request.args.get("l",None)
    shell_wrappers.speak(saying,lang)
    return f'OK: {saying}, language is: {lang}'

@app.route('/')
def return_index():
    return send_from_directory('static', "index.html")


if __name__ == "__main__":
    app.run(host='0.0.0.0',port="42069")