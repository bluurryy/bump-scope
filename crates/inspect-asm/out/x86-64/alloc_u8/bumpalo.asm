inspect_asm::alloc_u8::bumpalo:
	push rbx
	mov rcx, qword ptr [rdi + 16]
	mov rax, qword ptr [rcx + 32]
	dec rax
	cmp rax, qword ptr [rcx]
	jb .LBB_2
	mov qword ptr [rcx + 32], rax
.LBB_3:
	mov byte ptr [rax], sil
	pop rbx
	ret
.LBB_2:
	mov ebx, esi
	mov esi, 1
	mov edx, 1
	call qword ptr [rip + bumpalo::Bump::alloc_layout_slow@GOTPCREL]
	mov esi, ebx
	test rax, rax
	jne .LBB_3
	call qword ptr [rip + bumpalo::oom@GOTPCREL]