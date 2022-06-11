class GuiController{
    constructor(){
        console.log("guiController constructor called...")
        this.jsonGuiTree = this.getJsonGuiTree()
        this.defaultResponseHandler = null
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
        console.log("query:\n " + text);
    }

    updateRespDisp(text){
        this.jsonGuiTree
            .responseBox
            .textAreaRespDisp
            .innerHTML = text
            console.log("response:\n " + text);
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