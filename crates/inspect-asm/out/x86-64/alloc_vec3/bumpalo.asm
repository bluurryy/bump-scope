inspect_asm::alloc_vec3::bumpalo:
	push rbx
	mov rcx, qword ptr [rdi + 16]
	mov rax, qword ptr [rcx + 32]
	cmp rax, 12
	jb .LBB_3
	add rax, -12
	and rax, -4
	cmp rax, qword ptr [rcx]
	jb .LBB_3
	mov qword ptr [rcx + 32], rax
.LBB_4:
	mov ecx, dword ptr [rsi + 8]
	mov dword ptr [rax + 8], ecx
	mov rcx, qword ptr [rsi]
	mov qword ptr [rax], rcx
	pop rbx
	ret
.LBB_3:
	mov rbx, rsi
	mov esi, 4
	mov edx, 12
	call qword ptr [rip + bumpalo::Bump::alloc_layout_slow@GOTPCREL]
	mov rsi, rbx
	test rax, rax
	jne .LBB_4
	call qword ptr [rip + bumpalo::oom@GOTPCREL]