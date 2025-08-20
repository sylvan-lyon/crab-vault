# 🦀 CrabVault API 文档

[toc]

欢迎使用 CrabVault API！🗄️ 这是一个简洁、高效且类似 S3 的对象存储服务 API。

您将通过操作存储桶（Bucket）和对象（Object）的 URL 来管理它们的数据和元数据，它们的生命周期是绑定在一起的。

## 📖 基本概念

### 📍 API 入口点

所有 API 的基础 URL 是：
```
http://your-server-address:32767
```

### 🔐 认证

> **注意**：当前版本的 API **没有实现认证**。在生产环境部署前，请务必在代理层（如 Nginx）或通过中间件添加认证逻辑！

### 📝 自定义元数据

我们支持两种元数据：

1. **系统元数据**: 使用标准的 HTTP 头部，例如 `Content-Type`, `ETag` 等。

2. **用户元数据**: 您可以通过添加 `x-crab-meta-` 前缀的自定义请求头来存储您自己的键值对信息。

    * 例如：`x-crab-meta-user-id: 123`，`x-crab-meta-source: mobile-upload`。

    * 在响应中，这些元数据也会以相同的头部格式返回。

### ❌ 错误处理

如果请求出错，服务器会返回一个标准的 HTTP 错误状态码，响应体通常是一个包含错误信息的 JSON 对象，如：
```json
{
    "objectMetaNotFound": {
        "bucket": "sylvan",
        "object": "somefile"
    }
}
```

---

## 🪣 存储桶 (Bucket) 操作

存储桶是存放对象的容器。

### 1. 创建存储桶 (Create a Bucket)

创建一个新的存储桶来存放您的对象。此操作是幂等的。

* **Endpoint**: `PUT /{bucket_name}`
* **描述**: 如果存储桶不存在，则创建它。如果已存在，此操作不会产生任何影响。
* **路径参数**:
    * `bucket_name` (string, required): 您想要创建的存储桶的名称。
* **请求体** (可选): 一个 JSON 对象，用于存储该存储桶的用户元数据。
```json
{
    "owner": "team-alpha",
    "region": "cn-north-1"
}
```
* **成功响应**:
* `201 Created`: 存储桶被成功创建。
* **cURL 示例**:
```bash
# 创建一个名为 "my-awesome-bucket" 的存储桶并附加元数据
curl -X PUT http://localhost:3000/my-awesome-bucket \
    -H "Content-Type: application/json" \
    -d '{"project": "Project Phoenix"}'
```

### 2. 删除存储桶 (Delete a Bucket)

删除一个空的存储桶。

* **Endpoint**: `DELETE /{bucket_name}`
* **描述**: 只有当存储桶中没有任何对象时，才能成功删除。
* **成功响应**:
    * `204 No Content`: 存储桶被成功删除。
* **错误响应**:
    * `409 Conflict`: 如果存储桶不为空。
    * `404 Not Found`: 如果存储桶不存在。
* **cURL 示例**:
```bash
curl -X DELETE http://localhost:32767/my-awesome-bucket
```

---

## 📄 对象 (Object) 操作

对象是您存储在 CrabVault 中的基本数据单元。

### 1. 📤 上传/更新对象 (Upload/Update an Object)

上传一个新对象，或者用新数据完全替换一个已有的对象。

* **Endpoint**: `PUT /{bucket_name}/{*object_name}`
* **描述**: 将请求体中的数据作为对象内容进行存储，并从请求头中提取元数据。
* **路径参数**:
    * `bucket_name` (string, required): 对象所在的存储桶。
    * `object_name` (string, required): 对象的完整路径名，例如 `images/avatars/user123.jpg`。
* **请求头**:
    * `Content-Type` (string, optional): 对象的 MIME 类型，默认为 `application/octet-stream`。
    * `x-crab-meta-*` (string, optional): 任意数量的用户自定义元数据。
* **请求体**: 对象的原始二进制数据。
* **成功响应**:
    * `201 Created`: 对象被成功创建或更新。
* **cURL 示例**:
```bash
# 上传一个图片，并附带自定义元数据
curl -X PUT http://localhost:3000/my-awesome-bucket/photos/paris.jpg \
    -H "Content-Type: image/jpeg" \
    -H "x-crab-meta-author: John Doe" \
    -H "x-crab-meta-location: Paris" \
    --data-binary "@path/to/your/local/image.jpg"
```

### 2. 📥 下载对象 (Download an Object)

获取一个对象的完整数据和其所有元数据。

* **Endpoint**: `GET /{bucket_name}/{*object_name}`
* **描述**: 返回对象的元数据（在响应头中）和数据（在响应体中）。
* **成功响应**:
    * `200 OK`: 成功获取对象。响应头包含所有元数据，响应体是对象的数据。
* **cURL 示例**:
```bash
# 下载对象并显示响应头信息 (-v)
curl -v http://localhost:3000/my-awesome-bucket/photos/paris.jpg -o downloaded_paris.jpg
```
您将在终端输出中看到类似 `ETag`, `Content-Type`, `x-crab-meta-author` 等响应头。

### 3. 🔎 获取对象元数据 (Get Object Metadata)

仅获取一个对象的元数据，不下载其数据。非常适合用于检查对象状态。

* **Endpoint**: `HEAD /{bucket_name}/{*object_name}`
* **描述**: 和 `GET` 请求完全相同，但服务器 **不会** 返回响应体。
* **成功响应**:
    * `200 OK`: 响应头中包含了对象的全部元数据。
* **cURL 示例**:
```bash
# 使用 -I 选项来发送 HEAD 请求
curl -I http://localhost:3000/my-awesome-bucket/photos/paris.jpg
```

### 4. ✏️ 更新对象元数据 (Update Object Metadata)

在不重新上传整个对象数据的情况下，修改一个对象的 **用户元数据**。

* **Endpoint**: `PATCH /{bucket_name}/{*object_name}`
* **描述**: 请求体中的 JSON 对象将被合并到现有的用户元数据中。已有的键将被更新，新的键将被添加。
* **请求体**: 一个 JSON 对象。
```json
{
    "reviewed": "true",
    "location": "Eiffel Tower"
}
```
* **成功响应**:
    * `200 OK`: 元数据更新成功。
* **cURL 示例**:
```bash
curl -X PATCH http://localhost:3000/my-awesome-bucket/photos/paris.jpg \
-H "Content-Type: application/json" \
-d '{"reviewed": "true", "tags": "vacation,2025"}'
```

### 5. 🗑️ 删除对象 (Delete an Object)

从存储桶中永久删除一个对象及其所有元数据。

* **Endpoint**: `DELETE /{bucket_name}/{*object_name}`
* **描述**: 此操作是幂等的。删除一个不存在的对象也会返回成功。
* **成功响应**:
    * `204 No Content`: 对象被成功删除或本来就不存在。
* **cURL 示例**:
```bash
curl -X DELETE http://localhost:3000/my-awesome-bucket/photos/paris.jpg
```

---

## 🦌 列表操作

### 1. 获取所有桶的元数据 （List All Buckets Metadata）

获取所有桶的元数据

- **Endpoint**:`GET /`
- **描述**：此操作会将所有桶的元数据下载下来，以 JSON 列表的形式
- 成功响应：
    - `200 OK`：剩余的元数据会放在响应体中
- **cURL示例**

```bash
curl -v http://localhost:32767
```

- **响应示例**

```json
[
  {
    "meta": {
      "name": "some",
      "created-at": "2025-08-20T05:02:40.777065800Z",
      "updated-at": "2025-08-20T05:02:40.777068400Z",
      "user-meta": {}
    }
  },
  {
    "meta": {
      "name": "sylvan",
      "created-at": "2025-08-20T05:02:47.129737Z",
      "updated-at": "2025-08-20T05:02:47.129739800Z",
      "user-meta": {}
    }
  }
]
```

### 2. 获取某一个桶内所有对象的元数据

### 1. 获取所有桶的元数据 （List All Buckets Metadata）

获取所有桶的元数据

- **Endpoint**:`GET /{bucket_name}`
- **描述**：此操作会将指定桶内所有对象的元数据下载下来，以 JSON 列表的形式
- **成功响应**：
    - `200 OK`：剩余的元数据会放在响应体中
- **cURL示例**

```bash
curl -v http://localhost:32767/sylvan
```

- **响应示例**

```json
[
  {
    "object-name": "anotherfile.json",
    "bucket-name": "sylvan",
    "size": 22,
    "content-type": "application/json",
    "etag": "S9rLr0zoRYiZQquJ+Zcw1jRIp9gVItI55ZFhEpMExwk",
    "created-at": "2025-08-20T05:08:23.789410600Z",
    "updated-at": "2025-08-20T05:08:23.789411100Z",
    "user-meta": {
      "user": "alex"
    }
  },
  {
    "object-name": "somefile.json",
    "bucket-name": "sylvan",
    "size": 22,
    "content-type": "application/json",
    "etag": "S9rLr0zoRYiZQquJ+Zcw1jRIp9gVItI55ZFhEpMExwk",
    "created-at": "2025-08-20T05:02:13.464651600Z",
    "updated-at": "2025-08-20T05:02:13.464652600Z",
    "user-meta": {
      "user": "sylvan"
    }
  }
]
```

---
