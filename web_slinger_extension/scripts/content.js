

//testing with old project, will replace with the web slinger game

//todo: find way to add chrome.runtime url to fetch calls in game js file
const canvas = document.createElement("canvas");
canvas.id = "bevy";
canvas.tabIndex = 0;
canvas.style.width = "876px";
canvas.style.height = "911px";
canvas.style.minWidth = "180px";
canvas.style.minHeight = "120px";
canvas.setAttribute("alt", "App");
canvas.width = 876;
canvas.height = 911;

document.body.appendChild(canvas);

var styles = `
    #bevy { 
       position: fixed;
       top: 0px;
       right: 0px; 
    }
    
`
var styleSheet = document.createElement("style");
styleSheet.textContent = styles;
document.head.appendChild(styleSheet);

(async () => {
    const module = await import(chrome.runtime.getURL("game/out/web_slinger.js"));

    // module.default(); // If it's a named export, change to `module.init();`
    await module.default(chrome.runtime.getURL("./game/out/web_slinger_bg.wasm")); // If it's a named export, change to `module.init();`

})();
