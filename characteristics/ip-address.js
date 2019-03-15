let util = require('util')
let bleno = require('bleno')
let UUID = require('./sugar-uuid')

let BlenoCharacteristic = bleno.Characteristic

let IpAddressCharacteristic = function() {
  IpAddressCharacteristic.super_.call(this, {
    uuid: UUID.WIFI_NAME,
    properties: ['read', 'notify']
  })
}

util.inherits(IpAddressCharacteristic, BlenoCharacteristic)

IpAddressCharacteristic.prototype.onReadRequest = function(offset, callback) {
  let result = this.RESULT_SUCCESS
  let data = new Buffer('192.168.0.1')

  if (offset > data.length) {
    result = this.RESULT_INVALID_OFFSET
    data = null
  } else {
    data = data.slice(offset)
  }
  callback(result, data)
}

IpAddressCharacteristic.prototype.onSubscribe = function(maxValueSize, updateValueCallback) {
  console.log('IpAddressCharacteristic subscribe')

  this.counter = 0
  this.changeInterval = setInterval(function() {
    let data = new Buffer(4)
    data.writeUInt32LE(this.counter, 0)

    console.log('IpAddressCharacteristic update value: ' + this.counter)
    updateValueCallback(data)
    this.counter++
  }.bind(this), 5000)
}

IpAddressCharacteristic.prototype.onUnsubscribe = function() {
  console.log('IpAddressCharacteristic unsubscribe')

  if (this.changeInterval) {
    clearInterval(this.changeInterval)
    this.changeInterval = null
  }
}

IpAddressCharacteristic.prototype.onNotify = function() {
  console.log('IpAddressCharacteristic on notify')
}

module.exports = IpAddressCharacteristic