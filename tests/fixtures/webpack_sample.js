// Webpack bundle sample fixture
// This simulates a minimal webpack bundle for testing purposes
(function(modules) {
  var installedModules = {};
  function __webpack_require__(moduleId) {
    if (installedModules[moduleId]) {
      return installedModules[moduleId].exports;
    }
    var module = installedModules[moduleId] = {
      i: moduleId,
      l: false,
      exports: {}
    };
    modules[moduleId].call(module.exports, module, module.exports, __webpack_require__);
    module.l = true;
    return module.exports;
  }
  __webpack_require__.m = modules;
  __webpack_require__.c = installedModules;
  return __webpack_require__(__webpack_require__.s = 0);
})({
  0: function(module, exports, __webpack_require__) {
    var utils = __webpack_require__(1);
    var result = utils.add(1, 2);
    console.log("Result:", result);
    module.exports = { result };
  },
  1: function(module, exports) {
    exports.add = function(a, b) { return a + b; };
    exports.multiply = function(a, b) { return a * b; };
    exports.VERSION = "1.0.0";
  }
});
