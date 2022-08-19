class GuiController{
    constructor(){
        // console.log("guiController constructor called...")
        this.jsonGuiTree = this.getJsonGuiTree()
        this.defaultResponseHandler = null
        this.initPopulateButtons()
    }

    cbPopulateButtons(responseStatus,responseText){
        // console.log(responseText)
        var domContent = ""
        
        var buttonList = JSON.parse(responseText)

        // console.log(buttonList)

        for (const b in buttonList) {
            var butName = buttonList[b]

            domContent += "<div class=\"siimple-btn siimple-btn--orange\" \
            onclick=\"httpGet(null, 'play/" + butName + "',handleServerResponse)\">\
            " + butName + "</div>"
        }

        document.getElementById("audioFilesDisp").innerHTML = domContent
        document.getElementById("respDisp").innerHTML = "Button list retrieved."
    }


    initPopulateButtons(){
        var ButtonList = httpGet(this, "getFilelist",this.cbPopulateButtons)
        this.jsonGuiTree.audioFilesBox.innerHTML = "requesting audio files list..."
    }



    attachResponseHandler(handler){
        this.defaultResponseHandler = handler
    }

    getJsonGuiTree(){
        return {
            queryBox :{
                say :{
                    textInputParam : document.getElementById('param_say')
                },
                mow :{
                    textInputParam : document.getElementById('param_mow')
                },
                guess :{
                    textInputParam : document.getElementById('param_guess')
                },
                
            },
            queryDisplayBox :{
                textAreaQueryDisp : document.getElementById("queryDisp")
            },
            responseBox :{
                textAreaRespDisp : document.getElementById("respDisp")
            },
            antControlBox :{
                tagDeviceStatus : document.getElementById("deviceStatusTag")
            },
            audioFilesBox : document.getElementById("audioFilesDisp")
        }
    }

    getQueryFromQueryBox(queryType){
        return queryType + "/"
                + this.jsonGuiTree.queryBox[queryType]
                    .textInputParam.value;
    }


    updateIP(text){
        this.jsonGuiTree
            .queryBox
            .textInputIpAddr.value = text
    }

    updateQueryDisp(text){
        this.jsonGuiTree
            .queryDisplayBox
            .textAreaQueryDisp
            .innerHTML = text
        // console.log("query:\n " + text);
    }

    updateRespDisp(text){
        this.jsonGuiTree
            .responseBox
            .textAreaRespDisp
            .innerHTML = text
            // console.log("response:\n " + text);
    }

    drawDeviceStatus(statusText){
        var statusColor;
        switch (statusText){
            case "Online":
                statusColor = "success";
                break;
            case "Offline":
                statusColor = "dark"
                break;
            case "Error":
                statusColor = "error"
                break;
            default:
                statusText = "Unknown";
                statusColor = "light";
                break;
        }
        this.jsonGuiTree
            .antControlBox
            .tagDeviceStatus
            .innerHTML = statusText;
        this.jsonGuiTree
            .antControlBox
            .tagDeviceStatus
            .className = "siimple-tag siimple-tag--"
                         + statusColor
                         + " siimple-tag--rounded";
    }

}