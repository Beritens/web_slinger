

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
       z-index: 100000000;
    }
    
`
var styleSheet = document.createElement("style");
styleSheet.textContent = styles;
document.head.appendChild(styleSheet);


// const all = [...document.querySelectorAll('*')].filter(el => {
//     return el.childNodes.length && [...el.childNodes].some(node => node.nodeType === Node.TEXT_NODE && node.nodeValue.trim());
// });

// for (var i = 0, max = all.length; i < max; i++) {
//     console.log("test")
// }

//https://stackoverflow.com/questions/49751396/determine-if-element-is-behind-another
function isBehindOtherElement(element) {
    const boundingRect = element.getBoundingClientRect()
    // adjust coordinates to get more accurate results
    const left = boundingRect.left + 5
    const right = boundingRect.right - 5
    const top = boundingRect.top + 5
    const bottom = boundingRect.bottom - 5

    // if (!element.contains(document.elementFromPoint(left, top))) return true
    // if (!element.contains(document.elementFromPoint(right, top))) return true
    // if (!element.contains(document.elementFromPoint(left, bottom))) return true
    // if (!element.contains(document.elementFromPoint(right, bottom))) return true

    if (element.contains(getElAtPoint(left, top))) {
        console.log(element)
        return false;

    }
    if (element.contains(getElAtPoint(right, top))) {
        console.log(element)
        return false;

    }
    if (element.contains(getElAtPoint(left, bottom))) {
        console.log(element)
        return false;

    }
    if (element.contains(getElAtPoint(right, bottom))) {
        console.log(element)
        return false;

    }

    return true;
}

function getElAtPoint(x, y) {
    let caretPos = document.caretPositionFromPoint(x, y);

    if (!caretPos) return; // No range found (shouldn't happen, but safety check)

    let node = caretPos.offsetNode;

    // Ensure we're in a text node
    if (node.nodeType === Node.TEXT_NODE) {
        let text = node.textContent;

        // Create a range for just the single character at this index
        // let charRange = document.createRange();
        // charRange.setStart(node, 0);
        // charRange.setEnd(node, text.length);

        // Get the bounding box of the character
        // let rect = charRange.getBoundingClientRect();
        let rect = node.parentElement.getBoundingClientRect();

        // Check if the click is actually inside the bounding box
        if (
            x < rect.left || x > rect.right ||
            y < rect.top || y > rect.bottom
        ) {
            return null;
        }
        // Find the parent element containing this text node
        let element = node.parentElement;
        return element
    } else {
        return null;
    }
}

(async () => {

    window.get_colliders = function () {
        const gameScreen = document.querySelector('#bevy');
        const originalDisplay = gameScreen.style.display;
        gameScreen.style.display = 'none';

        const textNodes = [];

        document.querySelectorAll('*').forEach(el => {
            // Ignore non-visible elements
            const style = getComputedStyle(el);
            const rect = el.getBoundingClientRect();
            const isVisible = rect.width > 0 && rect.height > 0 &&
                style.visibility !== 'hidden' &&
                style.opacity !== 0 &&
                style.display !== 'none';
            const isBehind = isBehindOtherElement(el);

            if (!isBehind && isVisible && !['STYLE', 'SCRIPT', 'META', 'LINK', 'NOSCRIPT'].includes(el.tagName)) {
                el.childNodes.forEach(node => {
                    if (node.nodeType === Node.TEXT_NODE && node.nodeValue.trim()) {
                        textNodes.push(node);
                    }
                });
            }
        });

        // const rects = [];
        const colliders = []
        const scrollbarWidth = window.innerWidth - document.documentElement.clientWidth;

        textNodes.forEach(node => {
            var text = node.nodeValue

            let range = document.createRange();
            if (text && text.trim() && text.trim().length > 0) {
                for (let i = 0; i < text.length; i++) {
                    let char = text[i];
                    if (!char.trim()) continue;
                    range.setStart(node, i);
                    range.setEnd(node, i + 1);
                    const rect = range.getBoundingClientRect();
                    // rects.push(rect);
                    if (rect.width > 0 && rect.height > 0) { // Ensure valid rectangles
                        colliders.push({ top: rect.top, bottom: rect.bottom, right: rect.right + scrollbarWidth, left: rect.left + scrollbarWidth });
                    }
                    // colliders.push({ top: rect.top, bottom: rect.bottom, right: rect.right, left: rect.left });
                }

            }
        })

        gameScreen.style.display = originalDisplay;

        return colliders;

        // console.log(rects);
        // return [
        //     { top: 10, bottom: 0, right: 20, left: 5 },
        //     { top: 15, bottom: 5, right: 25, left: 10 }
        // ];
    };

    const module = await import(chrome.runtime.getURL("game/out/web_slinger.js"));

    // module.default(); // If it's a named export, change to `module.init();`
    await module.default(chrome.runtime.getURL("./game/out/web_slinger_bg.wasm")); // If it's a named export, change to `module.init();`




})();
