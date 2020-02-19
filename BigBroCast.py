from flask import Flask, escape, request
import os
import re

app = Flask(__name__)

config = {
    'max_sentence_len':5
}

def sanitize(s:str)-> str:
    return re.sub('[^0-9a-zA-Z,.?!\'żźćńółęąśŻŹĆĄŚĘŁÓŃ ]+', '', s) 

def speak_single_sentence(sentence_sane:str, lang_sane:str="en") -> None:
    command = f'shellscripts/speak.sh "{sentence_sane}" {lang_sane}'
    print("Calling command:" + command)
    os.system(command)

def speak(saying:str,lang:str="en") -> None:
    saying_sane = sanitize(saying)
    lang_sane = sanitize(lang)
    print(saying_sane)
    sentences_sane = re.split('[.?!]',saying_sane)[:10]
    print(sentences_sane)
    [speak_single_sentence(s,lang_sane)
        for s in sentences_sane
    ]

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

@app.route('/favicon.ico')
def serve_favicon():
    return 'too lazy.'

@app.route('/say/<saying>')
def say(saying:str):
    speak(saying,"en")
    return f'OK: {saying}'

@app.route('/mow/<saying>')
def mow(saying:str):
    speak(saying,"pl")
    return f'OK: {saying}'

@app.route('/vol/<int:vol_percent>')
def set_vol(vol_percent:int):
    if 0 <= vol_percent <= 100:
        os.system(f'shellscripts/set_vol.sh {vol_percent}')
        return f'volume set ok: {vol_percent}%'
    else:
        return f'invalid volume requested: {vol_percent}%'

@app.route('/<saying>')
def speak_any_lang(saying:str):
    lang = request.args.get("l","en")
    speak(saying,lang)
    return f'OK: {saying}, language is: {lang}'

app.run(host='0.0.0.0',port="42069")
