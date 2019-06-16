const formatTime = date => {
  const year = date.getFullYear()
  const month = date.getMonth() + 1
  const day = date.getDate()
  const hour = date.getHours()
  const minute = date.getMinutes()
  const second = date.getSeconds()

  return [year, month, day].map(formatNumber).join('/') + ' ' + [hour, minute, second].map(formatNumber).join(':')
}

const formatNumber = n => {
  n = n.toString()
  return n[1] ? n : '0' + n
}

function ab2str(buffer) {
  const hexArr = Array.prototype.map.call(
    new Uint8Array(buffer),
    function (bit) {
      return ('00' + bit.toString(16)).slice(-2)
    }
  )
  return hexCharCodeToStr(hexArr.join(''))
}

function str2ab(str) {
  let val = ""
  for (let i = 0; i < str.length; i++) {
    if (val === '') {
      val = str.charCodeAt(i).toString(16)
    } else {
      val += ',' + str.charCodeAt(i).toString(16)
    }
  }
  return new Uint8Array(val.match(/[\da-f]{2}/gi).map(function (h) {
    return parseInt(h, 16)
  })).buffer
}

function str2abs(str) {
  let val = ""
  for (let i = 0; i < str.length; i++) {
    if (val === '') {
      val = str.charCodeAt(i).toString(16)
    } else {
      val += ',' + str.charCodeAt(i).toString(16)
    }
  }
  let valArray = val.split(',')
  let len = valArray.length
  let bufferArray = []
  while (valArray.length > 0) {
    let value = valArray.splice(0, 20).join(',')
    bufferArray.push(new Uint8Array(value.match(/[\da-f]{2}/gi).map(function (h) {
      return parseInt(h, 16)
    })).buffer)
  } 
  return bufferArray
}

function buf2hex(buffer) {
  return Array.prototype.map.call(new Uint8Array(buffer), x => ('00' + x.toString(16)).slice(-2)).join('');
}

function apiAsync(api, options) {
  return new Promise(function (resolve, reject) {
    let success = function (res) {
      resolve(res)
    }
    let fail = function (err) {
      resolve(err)
    }
    wx[api]({ ...options, success, fail })
  })
}

function hexCharCodeToStr(hexCharCodeStr) {
  let trimedStr = hexCharCodeStr.trim();
  let rawStr =
    trimedStr.substr(0, 2).toLowerCase() === "0x"
      ?
      trimedStr.substr(2)
      :
      trimedStr;
  let len = rawStr.length;
  if (len % 2 !== 0) {
    console.log("Illegal Format ASCII Code!");
    return "";
  }
  let curCharCode;
  let resultStr = [];
  for (var i = 0; i < len; i = i + 2) {
    curCharCode = parseInt(rawStr.substr(i, 2), 16); // ASCII Code Value
    resultStr.push(String.fromCharCode(curCharCode));
  }
  return resultStr.join("");
}



module.exports = {
  formatTime: formatTime,
  ab2str,
  str2ab,
  str2abs,
  buf2hex,
  apiAsync,
  hexCharCodeToStr
}
