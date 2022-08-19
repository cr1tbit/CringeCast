from flask import Flask, escape, request, send_from_directory
import os
import re
import langdetect
import json

app = Flask(__name__,static_url_path="/static")

config = {
    'max_sentence_len':16,
    'max_api_str_len':200
}

def sanitize(s:str)-> str:
    return re.sub('[^0-9a-zA-Z,.?!\'żźćńółęąśŻŹĆĄŚĘŁÓŃ ]+', '', s) 

def smart_split(s:str) -> list:
    # since google voice API doesn't respond to strings longer than 200-chars
    # we're stripping longer strings into a couple of requests.
    s_return = []
    for ss in re.split('[.?!]',s):
        if len(ss) <= config['max_api_str_len']:
            s_return.append(ss)
        else:
            #think of something smarter! 
            s_return.append(ss[:config['max_api_str_len']])
    return s_return


def speak_single_sentence(sentence_sane:str, lang_sane:str="en") -> None:
    command = f'shellscripts/speak.sh "{sentence_sane}" {lang_sane}'
    # print("Calling command:" + command)
    os.system(command)

def speak(saying:str,lang:str) -> None:
    saying_sane = sanitize(saying)
    sentences_sane = smart_split(saying_sane)[:config['max_sentence_len']]
    if lang is None:
        try:
            lang_sane = langdetect.detect(sentences_sane[0])
        except langdetect.lang_detect_exception.LangDetectException:
            lang_sane = "en"
    else:
        lang_sane = sanitize(lang)
    [speak_single_sentence(s,lang_sane)
        for s in sentences_sane
    ]

def play_file(filename_no_ext:str) -> None:
    filename_sane = sanitize(filename_no_ext)+".mp3"
    command = f'shellscripts/play.sh audio_files/{filename_sane}'
    # print("Calling command:" + command)
    os.system(command) 

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

@app.route('/favicon.png')
def serve_favicon():
    return send_from_directory('static', "favicon.png")

@app.route('/say/<saying>')
def say(saying:str):
    speak(saying,"en")
    return f'OK: {saying}'

@app.route('/mow/<saying>')
def mow(saying:str):
    speak(saying,"pl")
    return f'OK: {saying}'

@app.route('/play/<filename>')
def play(filename:str):
    play_file(filename)
    return f'OK: {filename}.mp3'

@app.route('/getFilelist')
def getFilelist():

    audio_names = [f.split(".")[0] \
        for f in os.listdir("audio_files")\
        if f.split(".")[1] == "mp3"]

    return json.dumps(audio_names)

@app.route('/vol/<int:vol_percent>')
def set_vol(vol_percent:int):
    if 0 <= vol_percent <= 100:
        os.system(f'shellscripts/set_vol.sh {vol_percent}')
        return f'volume set ok: {vol_percent}%'
    else:
        return f'invalid volume requested: {vol_percent}%'

@app.route('/guess/<saying>')
@app.route('/<saying>')
def speak_any_lang(saying:str):
    lang = request.args.get("l",None)
    speak(saying,lang)
    return f'OK: {saying}, language is: {lang}'

@app.route('/')
def return_index():
    return send_from_directory('static', "index.html")

app.run(host='0.0.0.0',port="42069")