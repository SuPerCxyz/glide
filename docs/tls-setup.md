# TLS/HTTPS 配置指南

Glide 服务端本身不处理 TLS，通过反向代理（Nginx/Caddy）实现 HTTPS。

## Caddy（推荐，自动证书）

```
glide.example.com {
    reverse_proxy localhost:8080
}
```

## Nginx

```nginx
server {
    listen 443 ssl;
    server_name glide.example.com;
    
    ssl_certificate /etc/ssl/certs/glide.pem;
    ssl_certificate_key /etc/ssl/private/glide.key;
    
    location / {
        proxy_pass http://localhost:8080;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }
}
```

## 自签名证书（测试用）

```bash
openssl req -x509 -newkey rsa:4096 -keyout key.pem -out cert.pem -days 365 -nodes
```

## 客户端配置

- GUI：在"服务器地址"中输入 `https://your-domain.com`
- CLI：`glide --server https://your-domain.com copy "hello"`
- 环境变量：`GLIDE_PUBLIC_URL=https://your-domain.com`
