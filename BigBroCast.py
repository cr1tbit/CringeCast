from flask import Flask, escape, request
import os

app = Flask(__name__)

#please ~hack~ harden me :(

@app.route('/')
def speak_any_lang():
    saying = request.args.get("s", "test")
    lang = request.args.get("l","en")
    os.system(f'shellscripts/speak.sh "{saying}" {lang}')
    return f'OK:{saying}'

@app.route('/say')
def say():
    saying = request.args.get("s", "test")
    os.system(f'shellscripts/en_speak.sh {saying}')
    return f'OK:{saying}'

@app.route('/mow')
def mow():
    saying = request.args.get("s", "test")
    os.system(f'shellscripts/pl_speak.sh {saying}')
    return f'okej:{saying}'

app.run(host='0.0.0.0',port="42069")
