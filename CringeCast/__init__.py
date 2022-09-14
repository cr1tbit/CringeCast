from flask import Flask, escape, request, send_from_directory, g
# import werkzeug
import json

import CringeCast.shell_wrappers as shell_wrappers
import os

app = Flask(__name__,static_url_path="/static")
app.config['MAX_CONTENT_LENGTH'] = 1 * 1000 * 1000
app.super_secret_key="test"



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
# @app.before_request
# def assert_teapot_mode():
#     if g.in_teapot_mode == True:
#         return "I'm sorry, I'm a teapot ", 418

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