inspect_asm::alloc_u32::try_up:
	push rbx
	mov rcx, qword ptr [rdi]
	mov rax, qword ptr [rcx]
	mov rdx, qword ptr [rcx + 8]
	add rax, 3
	and rax, -4
	sub rdx, rax
	cmp rdx, 4
	jb .LBB_2
	lea rdx, [rax + 4]
	mov qword ptr [rcx], rdx
	test rax, rax
	je .LBB_2
.LBB_4:
	mov dword ptr [rax], esi
	pop rbx
	ret
.LBB_2:
	mov ebx, esi
	call bump_scope::bump_scope::BumpScope<A,_,_,_>::do_alloc_sized_in_another_chunk
	mov esi, ebx
	test rax, rax
	jne .LBB_4
	xor eax, eax
	pop rbx
	ret