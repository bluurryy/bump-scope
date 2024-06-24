inspect_asm::alloc_u8::bumpalo:
	push rbx
	mov rcx, qword ptr [rdi + 16]
	mov rax, qword ptr [rcx + 32]
	dec rax
	cmp rax, qword ptr [rcx]
	jb .LBB0_1
	mov qword ptr [rcx + 32], rax
	test rax, rax
	je .LBB0_1
.LBB0_0:
	mov byte ptr [rax], sil
	pop rbx
	ret
.LBB0_1:
	mov ebx, esi
	mov esi, 1
	mov edx, 1
	call qword ptr [rip + bumpalo::Bump::alloc_layout_slow@GOTPCREL]
	mov esi, ebx
	test rax, rax
	jne .LBB0_0
	call qword ptr [rip + bumpalo::oom@GOTPCREL]
