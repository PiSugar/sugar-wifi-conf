const execSync = require('child_process').execSync
let util = require('util')
let bleno = require('@stoprocent/bleno')
let UUID = require('../sugar-uuid')

let BlenoCharacteristic = bleno.Characteristic
let BlenoDescriptor = bleno.Descriptor

let modelBuffer = execSync('cat /proc/device-tree/model')

module.exports = new BlenoCharacteristic({
  uuid: UUID.DEVICE_MODEL,
    properties: ['read'],
    value: modelBuffer,
    descriptors: [
      new BlenoDescriptor({
        uuid: '2002',
        value: 'Raspberry Hardware Model'
      })
    ]
})
