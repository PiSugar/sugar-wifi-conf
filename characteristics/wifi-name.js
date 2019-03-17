const execSync = require('child_process').execSync
let util = require('util')
let bleno = require('bleno')
let UUID = require('../sugar-uuid')

let BlenoCharacteristic = bleno.Characteristic

let WifiNameCharacteristic = function() {
  WifiNameCharacteristic.super_.call(this, {
    uuid: UUID.WIFI_NAME,
    properties: ['read', 'notify']
  })
}

util.inherits(WifiNameCharacteristic, BlenoCharacteristic)

WifiNameCharacteristic.prototype.onReadRequest = function(offset, callback) {
  let result = this.RESULT_SUCCESS
  let data = new Buffer(getWifiName)

  if (offset > data.length) {
    result = this.RESULT_INVALID_OFFSET
    data = null
  } else {
    data = data.slice(offset)
  }
  callback(result, data)
}

WifiNameCharacteristic.prototype.onSubscribe = function(maxValueSize, updateValueCallback) {
  console.log('WifiNameCharacteristic subscribe')

  this.changeInterval = setInterval(function() {
    let wifi = getWifiName()
    let data = new Buffer(wifi)
    console.log('WifiNameCharacteristic update value: ' + wifi)
    updateValueCallback(data)
  }.bind(this), 5000)
}

WifiNameCharacteristic.prototype.onUnsubscribe = function() {
  console.log('WifiNameCharacteristic unsubscribe')

  if (this.changeInterval) {
    clearInterval(this.changeInterval)
    this.changeInterval = null
  }
}

WifiNameCharacteristic.prototype.onNotify = function() {
  console.log('WifiNameCharacteristic on notify')
}

function getWifiName() {
  const reg = /GENERAL\.CONNECTION:[\s]*([^\s]*)/
  let wifiBuffer = execSync('nmcli dev show wlan0')
  let wifiString = wifiBuffer.toString()
  let match = wifiString.match(reg)
  return match.length > 1 ? match[1] : ''
}

module.exports = WifiNameCharacteristic