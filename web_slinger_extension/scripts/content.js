

//testing with old project, will replace with the web slinger game

//todo: find way to add chrome.runtime url to fetch calls in game js file

(async () => {
    window.play_music = function (asset) {
        console.log("Playing music:", asset);
        // Call functions from the module if needed
    };

    window.stop_music = function () {
        console.log("Stopping music");
        // Call functions from the module if needed
    };

    const module = await import(chrome.runtime.getURL("test/out/not_worthy.js"));

    // module.default(); // If it's a named export, change to `module.init();`
    await module.default(chrome.runtime.getURL("./test/out/not_worthy_bg.wasm")); // If it's a named export, change to `module.init();`

})();
