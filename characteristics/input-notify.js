let util = require('util')
let bleno = require('bleno')
let UUID = require('./sugar-uuid')

let BlenoCharacteristic = bleno.Characteristic


// Input

let InputCharacteristic = function() {
  InputCharacteristic.super_.call(this, {
    uuid: UUID.INPUT,
    properties: ['write', 'writeWithoutResponse']
  })
}

util.inherits(InputCharacteristic, BlenoCharacteristic)

InputCharacteristic.prototype.onWriteRequest = function(data, offset, withoutResponse, callback) {
  console.log('InputCharacteristic write request: ' + data.toString('hex') + ' ' + offset + ' ' + withoutResponse)
  callback(this.RESULT_SUCCESS)
}

// NotifyMassage

let NotifyMassageCharacteristic = function() {
  NotifyMassageCharacteristic.super_.call(this, {
    uuid: UUID.NOTIFY_MESSAGE,
    properties: ['notify']
  })
}

util.inherits(NotifyMassageCharacteristic, BlenoCharacteristic)

NotifyMassageCharacteristic.prototype.onSubscribe = function(maxValueSize, updateValueCallback) {
  console.log('NotifyMassageCharacteristic subscribe')

  this.counter = 0
  this.changeInterval = setInterval(function() {
    let data = new Buffer(4)
    data.writeUInt32LE(this.counter, 0)

    console.log('NotifyMassageCharacteristic update value: ' + this.counter)
    updateValueCallback(data)
    this.counter++
  }.bind(this), 5000)
}

NotifyMassageCharacteristic.prototype.onUnsubscribe = function() {
  console.log('NotifyMassageCharacteristic unsubscribe')

  if (this.changeInterval) {
    clearInterval(this.changeInterval)
    this.changeInterval = null
  }
}

NotifyMassageCharacteristic.prototype.onNotify = function() {
  console.log('NotifyMassageCharacteristic on notify')
}

module.exports = {
  InputCharacteristic,
  NotifyMassageCharacteristic
}