const fs = require('fs')
const conf_path = './wpa_supplicant.conf'

function setWifi (input_ssid, input_password) {
	let data = fs.readFileSync(conf_path, 'utf8')
	let wifiRegx = /(network={[^\}]+})/g
	let ssidRegx = /ssid="([^"]*)"/
	let priorityRegx = /priority=([\d]*)/
	let wifiMatch = data.match(wifiRegx)
	let wifiArray = []
	let maxPriority = 0
	if (wifiMatch) {
		for (let i in wifiMatch) {
			let str = wifiMatch[i]
			let ssid = str.match(ssidRegx)
			ssid = ssid ? ssid[1] : ''
			let priority = str.match(priorityRegx)
			priority = priority ? priority[1] : 0
			maxPriority = Math.max(maxPriority, priority)
			if (input_ssid !== ssid) {
				wifiArray.push(str)
			}
			data = data.replace(wifiMatch[i], '')
		}
	}
	let prefix = data
	wifiArray.push(`network={\n\t\tssid="${input_ssid}"\n\t\tpsk="${input_password}"\n\t\tpriority=${maxPriority+1}\n\t}`)
	let content = `${prefix}\n\t${wifiArray.join('\n\t')}`
	console.log(content)
	fs.writeFileSync(conf_path, content)
}

setWifi ('ACL', '123456')
// fs.writeFileSync('./output.conf', '\n\t233')
