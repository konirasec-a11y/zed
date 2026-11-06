// Optional offline verification preview for environments without npm registry access.
// React + Vite sources remain authoritative; this renderer is not a production build.
const fs=require('fs'), path=require('path');
const globalTsPath=require('child_process').execFileSync('npm',['root','-g'],{encoding:'utf8'}).trim();
let ts; try { ts=require('typescript'); } catch { ts=require(path.join(globalTsPath,'typescript')); }
const root=path.resolve(__dirname,'..');
const all=[];
function scan(dir){for(const e of fs.readdirSync(dir,{withFileTypes:true})){const p=path.join(dir,e.name);if(e.isDirectory())scan(p);else if(/\.tsx?$/.test(e.name))all.push(p);}}
scan(path.join(root,'src'));
const transpiled = all.map(f=>{
 const id=path.relative(root,f).split(path.sep).join('/');
 const source=fs.readFileSync(f,'utf8');
 const output=ts.transpileModule(source,{fileName:f,compilerOptions:{target:ts.ScriptTarget.ES2019,module:ts.ModuleKind.CommonJS,jsx:ts.JsxEmit.React,esModuleInterop:true}}).outputText;
 return `__modules[${JSON.stringify(id)}] = function(require,module,exports){\n${output}\n};`;
}).join('\n');
const loader=`(function(){\nconst __modules={};const __cache={};\n${transpiled}\nfunction normalize(base,req){if(req==='react'||req==='react-dom/client')return req;if(req.endsWith('.css'))return 'css';let p=req[0]==='.'?base.split('/').slice(0,-1).concat(req.split('/')):req.split('/');let stack=[];for(let s of p){if(s==='.'||!s)continue;if(s==='..')stack.pop();else stack.push(s);}let x=stack.join('/');if(!/\\.tsx?$/.test(x))x+='.tsx';if(!__modules[x])x=x.replace(/\\.tsx$/,'.ts');return x;}\nfunction load(id){if(id==='react')return window.React;if(id==='react-dom/client')return {createRoot:function(el){return {render:function(node){window.ReactDOM.render(node,el)}}}};if(id==='css')return {};if(__cache[id])return __cache[id].exports;if(!__modules[id])throw new Error('Missing module: '+id);let module={exports:{}};__cache[id]=module;__modules[id](req=>load(normalize(id,req)),module,module.exports);return module.exports;}\nwindow.React.Fragment=window.React.Fragment||function(p){return p.children};window.React.StrictMode=window.React.StrictMode||window.React.Fragment;window.React.useState=window.React.useState||function(v){return [typeof v==='function'?v():v,function(){}]};load('src/main.tsx');})();`;
fs.writeFileSync(path.join(root,'preview','bundle.js'),loader);
const css=['tokens','base','app','diagrams','gallery'].map(x=>`<link rel="stylesheet" href="../src/styles/${x}.css"/>`).join('\n');
fs.writeFileSync(path.join(root,'preview','index.html'),`<!doctype html><html lang="en"><head><meta charset="utf-8"/><meta name="viewport" content="width=device-width,initial-scale=1"/><title>Model Studio — Offline Visual Preview</title>${css}</head><body><div id="root"></div><script src="runtime/react.js"></script><script src="runtime/react-dom.js"></script><script src="bundle.js"></script></body></html>`);
console.log('Offline preview generated. Open preview/index.html');
