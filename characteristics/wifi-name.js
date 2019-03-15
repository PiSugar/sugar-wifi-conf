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
  let data = new Buffer('dynamic value')

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

  this.counter = 0
  this.changeInterval = setInterval(function() {
    let data = new Buffer(4)
    data.writeUInt32LE(this.counter, 0)

    console.log('WifiNameCharacteristic update value: ' + this.counter)
    updateValueCallback(data)
    this.counter++
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

module.exports = WifiNameCharacteristic