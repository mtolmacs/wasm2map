import puppeteer from 'puppeteer';

(async () => {
    // Launch the browser and open a new blank page
    const browser = await puppeteer.launch({headless: false});
    const page = await browser.newPage();
  
    const client = await page.target().createCDPSession();
    client.on('Debugger.scriptParsed', () => {
        console.log(arguments)
    });
    await client.send('Page.navigate', {
        url: "http://localhost:8080"
    });

    //await browser.close();
  })();